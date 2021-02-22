mod settings;

#[macro_use]
extern crate serde_derive;

use crate::settings::Settings;
use either::*;
use env_logger::Env;
use log::{debug, info, trace, warn};
use paho_mqtt::{AsyncClient, ConnectOptionsBuilder, CreateOptionsBuilder, Message};
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
        let (len, _) = socket.recv_from(&mut buf).await?;

        if len > buf.len() {
            warn!("Receive buffer size is too low! ({} < {})", buf.len(), len);
        }
        trace!("Received buffer: {:?}", &buf[..]);

        if buf.iter().zip(old_buf.iter()).any(|(a, b)| a != b) {
            info!("Change in buffer...sending MQTT");
            old_buf.copy_from_slice(&buf);

            for tasmota in settings.tasmotas.iter() {
                let min_target_index = tasmota.mappings.iter().map(|m| m.target_start).min().unwrap();
                let max_target_index = tasmota.mappings.iter().map(|m| m.target_start + m.length.unwrap_or(1)).max().unwrap();

                let mut colors = vec![Option::<u32>::None; (max_target_index - min_target_index) as usize];

                for mapping in tasmota.mappings.iter() {
                    let range = if mapping.reverse.unwrap_or(false) {
                        Left((0..mapping.length.unwrap_or(1)).rev())
                    }
                    else {
                        Right(0..mapping.length.unwrap_or(1))
                    };

                    for i in range.into_iter() {
                        colors[(mapping.target_start - min_target_index + i) as usize] = Some(
                            buf[mapping.source_start as usize * 3 + i as usize * 3 + 2] as u32
                                + ((buf[mapping.source_start as usize * 3 + i as usize * 3 + 1]
                                as u32)
                                << 8)
                                + ((buf[mapping.source_start as usize * 3 + i as usize * 3]
                                as u32)
                                << 16)
                        )
                    }
                }
                let mut payload =
                    String::with_capacity((max_target_index - min_target_index) as usize * 8);

                colors.iter().for_each(|c| payload.push_str(&format!("#{:06x} ", c.unwrap_or(0))));

                debug!("MQTT payload for {}: {}", tasmota.mqtt_prefix, payload);

                mqtt_client
                    .publish(Message::new(
                        format!("{}/LED{}", tasmota.mqtt_prefix, min_target_index + 1),
                        payload,
                        1,
                    ))
                    .await?;
            }
        }
    }
}
