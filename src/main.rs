mod settings;

#[macro_use]
extern crate serde_derive;

use crate::settings::Settings;
use either::*;
use env_logger::Env;
use log::{debug, info, trace, warn};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();

    let settings = Settings::new()?;

    info!("{:?}", settings);

    let mut buf = vec![0; settings.clone().receive_buffer_size.unwrap_or(1024) as usize];

    let receive_socket = UdpSocket::bind(settings.clone().udp_bind_address).await?;
    let send_socket = UdpSocket::bind("0.0.0.0:12345").await?;

    let (tx_chan, mut rx_chan) = mpsc::channel::<Vec<u8>>(1_000);

    tokio::spawn(async move {
        while let Some(buf) = rx_chan.recv().await {
            for wled in settings.wleds.iter() {
                let mut packet = vec![0_u8; (wled.number_of_leds * 3 + 2) as usize];
                packet[0] = 2; // DRGB
                packet[1] = 1; // Timeout

                wled.mappings.iter().for_each(|mapping| {
                    let range = if mapping.reverse.unwrap_or(false) {
                        Left((0..mapping.length.unwrap_or(1)).rev())
                    } else {
                        Right(0..mapping.length.unwrap_or(1))
                    };

                    for (i, j) in range.into_iter().enumerate() {
                        let buffer_start_index = mapping.source_start as usize * 3 + i * 3;
                        let target_start_index =
                            (mapping.target_start as usize + j as usize) * 3 + 2;
                        packet[target_start_index..target_start_index + 3]
                            .copy_from_slice(&buf[buffer_start_index..buffer_start_index + 3]);
                    }
                });

                debug!("UDP payload: {:?}", packet);
                send_socket
                    .send_to(packet.as_slice(), &wled.url)
                    .await
                    .unwrap();
            }
        }
    });

    loop {
        let (len, _) = receive_socket.recv_from(&mut buf).await?;

        if len == buf.len() {
            warn!("Receive buffer size might be too low!");
        }
        trace!("Received buffer: {:?}", &buf[..]);
        tx_chan.send(buf[..len].to_vec()).await?;
    }
}
