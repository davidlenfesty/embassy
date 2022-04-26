use core::convert::TryFrom;

use super::{set_freqs, Clocks};
use crate::pac::flash::vals::Latency;
use crate::pac::gpio::vals::{CnfOut, Mode};
use crate::pac::rcc::vals::{
    Adcpre, Hpre, Mco, Pll2mul, Pllmul, Pllsrc, Ppre1, Prediv1, Prediv1src, Sw, Usbpre,
};
use crate::pac::{FLASH, GPIOA, RCC};
use crate::time::Hertz;

use core::ops::{Div, Mul};

const HSI: u32 = 8_000_000;

/// Enum for configuring PREDIV1 and PREDIV2
#[derive(Clone, Copy, PartialEq)]
pub enum Prediv {
    NotDivided,
    Div2,
    Div3,
    Div4,
    Div5,
    Div6,
    Div7,
    Div8,
    Div9,
    Div10,
    Div11,
    Div12,
    Div13,
    Div14,
    Div15,
    Div16,
}

impl Div<Prediv> for Hertz {
    type Output = Hertz;

    fn div(self, rhs: Prediv) -> Self::Output {
        let divisor = match rhs {
            Prediv::NotDivided => 1,
            Prediv::Div2 => 2,
            Prediv::Div3 => 3,
            Prediv::Div4 => 4,
            Prediv::Div5 => 5,
            Prediv::Div6 => 6,
            Prediv::Div7 => 7,
            Prediv::Div8 => 8,
            Prediv::Div9 => 9,
            Prediv::Div10 => 10,
            Prediv::Div11 => 11,
            Prediv::Div12 => 12,
            Prediv::Div13 => 13,
            Prediv::Div14 => 14,
            Prediv::Div15 => 15,
            Prediv::Div16 => 16,
        };
        Hertz(self.0 / divisor)
    }
}

impl Into<Prediv1> for Prediv {
    fn into(self) -> Prediv1 {
        match self {
            Prediv::NotDivided => Prediv1::DIV1,
            Prediv::Div2 => Prediv1::DIV2,
            Prediv::Div3 => Prediv1::DIV3,
            Prediv::Div4 => Prediv1::DIV4,
            Prediv::Div5 => Prediv1::DIV5,
            Prediv::Div6 => Prediv1::DIV6,
            Prediv::Div7 => Prediv1::DIV7,
            Prediv::Div8 => Prediv1::DIV8,
            Prediv::Div9 => Prediv1::DIV9,
            Prediv::Div10 => Prediv1::DIV10,
            Prediv::Div11 => Prediv1::DIV11,
            Prediv::Div12 => Prediv1::DIV12,
            Prediv::Div13 => Prediv1::DIV13,
            Prediv::Div14 => Prediv1::DIV14,
            Prediv::Div15 => Prediv1::DIV15,
            Prediv::Div16 => Prediv1::DIV16,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum PllMul {
    Mul4,
    Mul5,
    Mul6,
    Mul7,
    Mul8,
    Mul9,
    Mul6_5,
}

impl Mul<PllMul> for Hertz {
    type Output = Hertz;

    fn mul(self, rhs: PllMul) -> Self::Output {
        let factors = match rhs {
            PllMul::Mul4 => (4, 1),
            PllMul::Mul5 => (5, 1),
            PllMul::Mul6 => (6, 1),
            PllMul::Mul7 => (7, 1),
            PllMul::Mul8 => (8, 1),
            PllMul::Mul9 => (9, 1),
            PllMul::Mul6_5 => (13, 2),
        };
        Hertz(self.0 * factors.0 / factors.1)
    }
}

impl Into<Pllmul> for PllMul {
    fn into(self) -> Pllmul {
        match self {
            PllMul::Mul4 => Pllmul::MUL4,
            PllMul::Mul5 => Pllmul::MUL5,
            PllMul::Mul6 => Pllmul::MUL6,
            PllMul::Mul7 => Pllmul::MUL7,
            PllMul::Mul8 => Pllmul::MUL8,
            PllMul::Mul9 => Pllmul::MUL9,
            // TODO fix clocks up properly to avoid this hackiness
            PllMul::Mul6_5 => Pllmul::MUL15,
        }
    }
}

/// Enum for configuring PLL2 and PLL3 multiplication factors.
#[derive(Clone, Copy, PartialEq)]
pub enum Pll2Mul {
    Mul8,
    Mul9,
    Mul10,
    Mul11,
    Mul12,
    Mul13,
    Mul14,
    Mul16,
    Mul20,
}

impl Mul<Pll2Mul> for Hertz {
    type Output = Hertz;

    fn mul(self, rhs: Pll2Mul) -> Self::Output {
        let factor = match rhs {
            Pll2Mul::Mul8 => 8,
            Pll2Mul::Mul9 => 9,
            Pll2Mul::Mul10 => 10,
            Pll2Mul::Mul11 => 11,
            Pll2Mul::Mul12 => 12,
            Pll2Mul::Mul13 => 13,
            Pll2Mul::Mul14 => 14,
            Pll2Mul::Mul16 => 16,
            Pll2Mul::Mul20 => 20,
        };
        Hertz(self.0 * factor)
    }
}

impl Into<Pll2mul> for Pll2Mul {
    fn into(self) -> Pll2mul {
        match self {
            Pll2Mul::Mul8 => Pll2mul::MUL8,
            Pll2Mul::Mul9 => Pll2mul::MUL9,
            Pll2Mul::Mul10 => Pll2mul::MUL10,
            Pll2Mul::Mul11 => Pll2mul::MUL11,
            Pll2Mul::Mul12 => Pll2mul::MUL12,
            Pll2Mul::Mul13 => Pll2mul::MUL13,
            Pll2Mul::Mul14 => Pll2mul::MUL14,
            Pll2Mul::Mul16 => Pll2mul::MUL16,
            Pll2Mul::Mul20 => Pll2mul::MUL20,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Prediv1Src {
    Hse,
    Pll2,
}

impl Into<Prediv1src> for Prediv1Src {
    fn into(self) -> Prediv1src {
        match self {
            Prediv1Src::Hse => Prediv1src::HSE,
            Prediv1Src::Pll2 => Prediv1src::PLL2,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum PllSrc {
    HsiDiv2,
    Prediv1,
}

impl Into<Pllsrc> for PllSrc {
    fn into(self) -> Pllsrc {
        match self {
            PllSrc::HsiDiv2 => Pllsrc::HSI_DIV2,
            PllSrc::Prediv1 => Pllsrc::HSE_DIV_PREDIV,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SysclockSrc {
    Hsi,
    Hse,
    Pll,
}

impl Into<Sw> for SysclockSrc {
    fn into(self) -> Sw {
        match self {
            SysclockSrc::Hsi => Sw::HSI,
            SysclockSrc::Hse => Sw::HSE,
            SysclockSrc::Pll => Sw::PLL,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum McoSrc {
    Hse,
    Hsi,
    Sysclk,
    PllClkDiv2,
    Pll2Clk,
    Pll3ClkDiv3,
    Pll3Clk,
    Xt1,
}

impl Into<Mco> for McoSrc {
    fn into(self) -> Mco {
        // TODO map properly
        match self {
            McoSrc::Hse => Mco::HSE,
            McoSrc::Hsi => Mco::HSI,
            McoSrc::Sysclk => Mco::SYSCLK,
            McoSrc::PllClkDiv2 => Mco::PLL,
            _ => Mco::NOMCO,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum AHBPrescaler {
    NotDivided,
    Div2,
    Div4,
    Div8,
    Div16,
    Div64,
    Div128,
    Div256,
    Div512,
}

impl Div<AHBPrescaler> for Hertz {
    type Output = Hertz;

    fn div(self, rhs: AHBPrescaler) -> Self::Output {
        let divisor = match rhs {
            AHBPrescaler::NotDivided => 1,
            AHBPrescaler::Div2 => 2,
            AHBPrescaler::Div4 => 4,
            AHBPrescaler::Div8 => 8,
            AHBPrescaler::Div16 => 16,
            AHBPrescaler::Div64 => 64,
            AHBPrescaler::Div128 => 128,
            AHBPrescaler::Div256 => 256,
            AHBPrescaler::Div512 => 512,
        };
        Hertz(self.0 / divisor)
    }
}

impl Into<Hpre> for AHBPrescaler {
    fn into(self) -> Hpre {
        match self {
            AHBPrescaler::NotDivided => Hpre::DIV1,
            AHBPrescaler::Div2 => Hpre::DIV2,
            AHBPrescaler::Div4 => Hpre::DIV4,
            AHBPrescaler::Div8 => Hpre::DIV8,
            AHBPrescaler::Div16 => Hpre::DIV16,
            AHBPrescaler::Div64 => Hpre::DIV64,
            AHBPrescaler::Div128 => Hpre::DIV128,
            AHBPrescaler::Div256 => Hpre::DIV256,
            AHBPrescaler::Div512 => Hpre::DIV512,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum APBPrescaler {
    NotDivided,
    Div2,
    Div4,
    Div8,
    Div16,
}

impl Div<APBPrescaler> for Hertz {
    type Output = Hertz;

    fn div(self, rhs: APBPrescaler) -> Self::Output {
        let divisor = match rhs {
            APBPrescaler::NotDivided => 1,
            APBPrescaler::Div2 => 2,
            APBPrescaler::Div4 => 4,
            APBPrescaler::Div8 => 8,
            APBPrescaler::Div16 => 16,
        };
        Hertz(self.0 / divisor)
    }
}

impl Into<Ppre1> for APBPrescaler {
    fn into(self) -> Ppre1 {
        match self {
            APBPrescaler::NotDivided => Ppre1::DIV1,
            APBPrescaler::Div2 => Ppre1::DIV2,
            APBPrescaler::Div4 => Ppre1::DIV4,
            APBPrescaler::Div8 => Ppre1::DIV8,
            APBPrescaler::Div16 => Ppre1::DIV16,
        }
    }
}

/// Configuration of the clocks
///
#[non_exhaustive]
#[derive(Default)]
pub struct Config {
    pub hse: Option<Hertz>,
    pub prediv1src: Option<Prediv1Src>,
    pub prediv1: Option<Prediv>,
    pub pllmul: Option<PllMul>,
    pub pllsrc: Option<PllSrc>,
    pub prediv2: Option<Prediv>,
    pub pll2mul: Option<Pll2Mul>,
    pub pll3mul: Option<Pll2Mul>,

    pub sysclk_src: Option<SysclockSrc>,

    pub mco_src: Option<McoSrc>,

    pub hpre: Option<AHBPrescaler>,
    pub ppre1: Option<APBPrescaler>,
    pub ppre2: Option<APBPrescaler>,
}

pub(crate) unsafe fn init(config: Config) {
    // TODO convert to "new style" config, use F2 as reference.
    // for now just use this bodge

    // Enable HSE if selected
    if let Some(hse) = config.hse {
        RCC.cr().modify(|w| w.set_hseon(true));
        while !RCC.cr().read().hsirdy() {}
    }

    if let Some(prediv2) = config.prediv2 {
        assert!(config.hse.is_some());

        RCC.cfgr2().modify(|w| w.set_prediv2(prediv2.into()));
    }

    // PLL2 and PLL3 should be configured prior to PLL
    let pll2clk = config.pll2mul.map(|pll2mul| {
        // TODO do we want to assert to check if PREDIVs are set?
        // Not *technically* necessary but I feel it might catch some dumb mistakes
        assert!(config.prediv2.is_some());
        // If not we definitely have to assert this
        assert!(config.hse.is_some());

        RCC.cfgr2().modify(|w| w.set_pll2mul(pll2mul.into()));
        RCC.cr().modify(|w| w.set_pll2on(true));
        while !RCC.cr().read().pll2rdy() {}

        config.hse.unwrap() / config.prediv2.unwrap() * pll2mul
    });

    let pll3clk = config.pll3mul.map(|pll3mul| {
        assert!(config.prediv2.is_some());
        assert!(config.hse.is_some());

        RCC.cfgr2().modify(|w| w.set_pll3mul(pll3mul.into()));
        RCC.cr().modify(|w| w.set_pll3on(true));
        while !RCC.cr().read().pll3rdy() {}

        config.hse.unwrap() / config.prediv2.unwrap() * pll3mul
    });

    let prediv1clk = config.prediv1.map(|prediv1| {
        assert!(config.prediv1src.is_some());

        let in_freq = match config.prediv1src.unwrap() {
            Prediv1Src::Hse => config.hse.unwrap(),
            Prediv1Src::Pll2 => pll2clk.unwrap(),
        };

        RCC.cfgr2().modify(|w| {
            w.set_prediv1src(config.prediv1src.unwrap().into());
            w.set_prediv1(prediv1.into());
        });

        in_freq / prediv1
    });

    // Configure PLL and start PLL
    let pllclk = config.pllmul.map(|pllmul| {
        let in_freq = match config.pllsrc.unwrap() {
            PllSrc::HsiDiv2 => Hertz(4_000_000), // 8MHz / 2
            PllSrc::Prediv1 => prediv1clk.unwrap(),
        };

        RCC.cfgr().modify(|w| {
            w.set_pllsrc(config.pllsrc.unwrap().into());
            w.set_pllmul(pllmul.into());
        });
        RCC.cr().modify(|w| w.set_pllon(true));
        while !RCC.cr().read().pllrdy() {}

        in_freq * pllmul
    });

    // Get SYSCLK frequency
    let sysclk = config
        .sysclk_src
        .map_or(Hertz(8_000_000), |sysclk_src| match sysclk_src {
            SysclockSrc::Hsi => Hertz(8_000_000),
            SysclockSrc::Hse => config.hse.unwrap(),
            SysclockSrc::Pll => pllclk.unwrap(),
        });

    // Only needed for stm32f10x
    FLASH.acr().write(|w| {
        w.set_latency(if sysclk <= Hertz(24_000_000) {
            Latency(0b000)
        } else if sysclk <= Hertz(48_000_000) {
            Latency(0b001)
        } else {
            Latency(0b010)
        });
    });

    // Configure peripheral clocks
    let ahbclk = match config.hpre {
        Some(hpre) => {
            RCC.cfgr().modify(|w| w.set_hpre(hpre.into()));
            sysclk / hpre
        }
        None => sysclk,
    };

    let (apb1clk, apb1clk_tim) = match config.ppre1 {
        Some(ppre1) => {
            RCC.cfgr().modify(|w| w.set_ppre1(ppre1.into()));
            let apb1clk = ahbclk / ppre1;
            let mul = match ppre1 {
                APBPrescaler::NotDivided => 1,
                _ => 2,
            };
            (apb1clk, Hertz(apb1clk.0 * mul))
        }
        None => (ahbclk, ahbclk),
    };

    let apb2clk = match config.ppre2 {
        Some(ppre2) => {
            RCC.cfgr().modify(|w| w.set_ppre2(ppre2.into()));
            let apb2clk = ahbclk / ppre2;
            let mul = match ppre2 {
                APBPrescaler::NotDivided => 1,
                _ => 2,
            };
            (apb2clk, Hertz(apb2clk.0 * mul))
        }
        None => (ahbclk, ahbclk),
    };

    // Check final clock frequencies
    assert!(sysclk <= Hertz(72_000_000));
    assert!(apb1clk <= Hertz(36_000_000));

    // Select MCO input
    let mco = config.mco_src.map_or(sysclk, |mco_src| match mco_src {
        // TODO proper type mappings and make everything work
        McoSrc::Hse => {
            RCC.cfgr().modify(|w| w.set_mco(Mco(0x06)));
            config.hse.unwrap()
        }
        McoSrc::Xt1 => {
            RCC.cfgr().modify(|w| w.set_mco(Mco(0b1010)));
            config.hse.unwrap()
        }
        McoSrc::Hsi => Hertz(8_000_000),
        McoSrc::Sysclk => sysclk,
        McoSrc::PllClkDiv2 => Hertz(pllclk.unwrap().0 / 2),
        McoSrc::Pll2Clk => pll2clk.unwrap(),
        McoSrc::Pll3ClkDiv3 => Hertz(pll3clk.unwrap().0 / 3),
        McoSrc::Pll3Clk => pll3clk.unwrap(),
    });

    // Finally switch over system clock
    if let Some(sysclk_src) = config.sysclk_src {
        RCC.cfgr().modify(|w| w.set_sw(sysclk_src.into()));
    }

    // TODO set these properly
    // TODO adcclk
    // TODO other clocks too
    set_freqs(Clocks {
        sys: Hertz(72_000_000),
        apb1: Hertz(36_000_000),
        apb2: Hertz(36_000_000),
        apb1_tim: Hertz(72_000_000),
        apb2_tim: Hertz(72_000_000),
        ahb1: Hertz(72_000_000),
        adc: Hertz(36_000_000), // TODO not necessarily correct, need to check if doing ADC stuff
    });
}
