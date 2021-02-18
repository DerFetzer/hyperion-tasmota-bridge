mod settings;

#[macro_use]
extern crate serde_derive;

use crate::settings::Settings;
use either::*;
use env_logger::Env;
use log::{debug, info};
use paho_mqtt::{AsyncClient, ConnectOptionsBuilder, CreateOptionsBuilder, Message};
use std::cmp;
use std::time::Duration;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();

    let settings = Settings::new()?;

    let mut buf = vec![0; settings.receive_buffer_size.unwrap_or(1024) as usize];
    let mut old_buf = vec![0; settings.receive_buffer_size.unwrap_or(1024) as usize];

    let socket = UdpSocket::bind(settings.udp_bind_address).await?;

    let create_opts = CreateOptionsBuilder::new()
        .server_uri(settings.mqtt.url)
        .client_id(settings.mqtt.client_id)
        .finalize();

    let mut conn_opts_builder = ConnectOptionsBuilder::new();

    conn_opts_builder.automatic_reconnect(Duration::new(5, 0), Duration::new(10, 0));

    if settings.mqtt.user.is_some() {
        conn_opts_builder.user_name(settings.mqtt.user.unwrap());
        conn_opts_builder.password(settings.mqtt.password.unwrap_or(String::new()));
    }

    let conn_opts = conn_opts_builder.finalize();

    let mqtt_client = AsyncClient::new(create_opts).unwrap();
    mqtt_client.connect(conn_opts).await?;

    loop {
        let (_, _) = socket.recv_from(&mut buf).await?;

        debug!(
            "Buffer start: {:?}",
            &buf[..cmp::min(settings.receive_buffer_size.unwrap_or(1024), 15) as usize]
        );

        if buf.iter().zip(old_buf.iter()).any(|(a, b)| a != b) {
            info!("Change in buffer...sending MQTT");
            old_buf.copy_from_slice(&buf);

            for tasmota in settings.tasmotas.iter() {
                for mapping in tasmota.mappings.iter() {
                    let mut payload =
                        String::with_capacity(mapping.length.unwrap_or(1) as usize * 8);

                    let range = if mapping.reverse.unwrap_or(false) {
                        Left((0..mapping.length.unwrap_or(1)).rev())
                    }
                    else {
                        Right(0..mapping.length.unwrap_or(1))
                    };

                    for i in range.into_iter() {
                        payload.push_str(&format!(
                            "#{:06x} ",
                            buf[mapping.source_start as usize * 3 + i as usize * 3 + 2] as u32
                                + ((buf[mapping.source_start as usize * 3 + i as usize * 3 + 1]
                                    as u32)
                                    << 8)
                                + ((buf[mapping.source_start as usize * 3 + i as usize * 3]
                                    as u32)
                                    << 16)
                        ))
                    }

                    mqtt_client
                        .publish(Message::new(
                            format!("{}/LED{}", tasmota.mqtt_prefix, mapping.target_start + 1),
                            payload,
                            1,
                        ))
                        .await?;
                }
            }
        }
    }
}
