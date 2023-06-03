use defmt::info;
use embassy_futures::select::{self, Either};
use embassy_rp::{
    gpio::{Input, Level, Output},
    peripherals::{PIN_16, PIN_17, PIN_18, PIN_20, PIN_28, PWM_CH1},
    pwm,
};

#[embassy_executor::task]
pub async fn buzzer_task(
    pwmch1: PWM_CH1,
    p16: PIN_16,
    p17: PIN_17,
    p18: PIN_18,
    p20: PIN_20,
    p28: PIN_28,
) {
    // let a = embassy_rp::gpio::OutputOpenDrain::new(p.PIN_8, Level::High);
    // let mut buzzer = Output::new(p18, Level::High);
    let mut button = Input::new(p20, embassy_rp::gpio::Pull::Down);
    let mut ledpin = Output::new(p28, Level::High);

    let mut rotary_a = Input::new(p16, embassy_rp::gpio::Pull::Up);
    let mut rotary_b = Input::new(p17, embassy_rp::gpio::Pull::Up);

    let mut pwm_config = pwm::Config::default();
    pwm_config.top = 0xffff;
    pwm_config.compare_a = 0x00ef;

    let mut pwm = pwm::Pwm::new_output_a(pwmch1, p18, pwm_config.clone());
    let mut control_volume = true;

    info!("Buzzer set up");
    loop {
        let v = select::select(
            async {
                button.wait_for_low().await;
                button.wait_for_high().await;
                control_volume = !control_volume;
                info!("Control volume = {}", control_volume);
            },
            async {
                let r = select::select(
                    async {
                        rotary_a.wait_for_rising_edge().await;
                    },
                    async {
                        rotary_b.wait_for_rising_edge().await;
                    },
                )
                .await;
                match r {
                    Either::First(()) => {
                        if rotary_b.is_high() {
                            info!("turning left");
                            return (0xff, false);
                        }
                        (0, false)
                    }
                    Either::Second(()) => {
                        if rotary_a.is_high() {
                            info!("turning right");
                            return (0xff, true);
                        }
                        (0, true)
                    }
                }
            },
        )
        .await;
        match v {
            Either::First(_) => {}
            Either::Second((v, add)) => match (control_volume, add) {
                (true, true) => {
                    pwm_config.compare_a += v;
                }
                (true, false) => {
                    pwm_config.compare_a -= v;
                }
                (false, true) => {
                    pwm_config.top += v;
                }
                (false, false) => {
                    pwm_config.top -= v;
                }
            },
        };
        if control_volume {
            info!("vol: {}", pwm_config.compare_a);
        } else {
            info!("top: {}", pwm_config.top);
        }

        pwm.set_config(&pwm_config);
    }
}
