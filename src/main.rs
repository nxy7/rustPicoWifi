#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

pub mod buzzer;
use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, DhcpConfig, Stack, StackResources};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_25, PIO0};
use embassy_rp::pio::Pio;
use embassy_time::Duration;
use embedded_io::asynch::Write;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

macro_rules! singleton {
    ($val:expr) => {{
        type T = impl Sized;
        static STATIC_CELL: StaticCell<T> = StaticCell::new();
        STATIC_CELL.init_with(move || $val)
    }};
}

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<
        'static,
        Output<'static, PIN_23>,
        PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>,
    >,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting program");

    let network_id = "internet xD";
    let network_password = "QwerFdsa";

    let p = embassy_rp::init(Default::default());
    // unwrap!(spawner.spawn(buzzer::buzzer_task(
    //     p.PWM_CH1, p.PIN_16, p.PIN_17, p.PIN_18, p.PIN_20, p.PIN_28
    // )));

    let fw = include_bytes!("../firmware/43439A0.bin");
    let clm = include_bytes!("../embassy/cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);

    // let led = Output::new(p.)
    let mut pio = Pio::new(p.PIO0);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    let state = singleton!(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;

    unwrap!(spawner.spawn(wifi_task(runner)));
    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::Dhcp(Default::default());

    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.
    let stack = &*singleton!(Stack::new(
        net_device,
        config,
        singleton!(StackResources::<2>::new()),
        seed
    ));

    unwrap!(spawner.spawn(net_task(stack)));
    info!("Spawned net task");

    let mut alarm_device_pin = Output::new(p.PIN_7, Level::Low);
    let mut beemo_l_eye_pin = Output::new(p.PIN_8, Level::Low);
    let mut beemo_r_eye_pin = Output::new(p.PIN_9, Level::Low);

    loop {
        match control.join_wpa2(network_id, network_password).await {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status={}", err.status);
            }
        }
    }

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    control.gpio_set(0, true).await;
    let ok_msg = r#"
HTTP/1.1 200 OK
Content-Length: 2
Content-Type: text/plain

ok"#;
    let err_msg = r#"
HTTP/1.1 400 Bad Request
Content-Length: 3
Content-Type: text/plain

err"#;

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        info!("Listening on TCP:1234...");
        if let Err(e) = socket.accept(1234).await {
            warn!("accept error: {:?}", e);
            continue;
        }

        info!("Received connection from {:?}", socket.remote_endpoint());

        loop {
            let _ = match socket.read(&mut buf).await {
                Ok(0) => {
                    warn!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    warn!("read error: {:?}", e);
                    break;
                }
            };
            let mut headers = [httparse::EMPTY_HEADER; 64];
            let mut req = httparse::Request::new(&mut headers);
            req.parse(&buf).unwrap();
            let Some(path)= req.path else {
                warn!("No path");
                break;
            };

            let res_status = match path {
                "/on" => {
                    control.gpio_set(0, true).await;
                    alarm_device_pin.set_high();
                    beemo_l_eye_pin.set_high();
                    beemo_r_eye_pin.set_high();
                    socket.write_all(ok_msg.as_bytes()).await
                }
                "/off" => {
                    control.gpio_set(0, false).await;
                    alarm_device_pin.set_low();
                    beemo_l_eye_pin.set_low();
                    beemo_r_eye_pin.set_low();
                    socket.write_all(ok_msg.as_bytes()).await
                }
                _ => socket.write_all(err_msg.as_bytes()).await,
            };

            if res_status.is_err() {
                warn!("err");
                break;
            }
        }
    }
}
