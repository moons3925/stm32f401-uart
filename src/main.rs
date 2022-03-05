#![no_std]
#![no_main]

use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
use cortex_m_rt::entry;
use stm32f4::stm32f401;

#[entry]
fn main() -> ! {
    let dp = stm32f401::Peripherals::take().unwrap();   // (1)デバイス用Peripheralsの取得
    clock_init(&dp);    // (2)クロック関連の初期化
    gpioa2a3_init(&dp); // (3)GPIOAの初期化
    usart2_init(&dp);   // (4)usart2の初期化
    loop {
        while dp.USART2.sr.read().rxne().bit() {	// (5)
            if dp.USART2.sr.read().pe().bit() {	// (6)parity error
                let err = b"\r\nDetected parity error.\r\n";    // (7)エラー検出時に送信する文字列
                let _ = dp.USART2.dr.read().bits(); // (8)読み捨てる
                for c in err.iter() {   // (9)文字列の送信処理
                    let data: u32 = *c as u32;
                    dp.USART2.dr.write( |w| unsafe { w.bits(data) });
                    while !dp.USART2.sr.read().txe().bit() {}   // (10)送り終わるまで待つ
                }
            }
            else if dp.USART2.sr.read().fe().bit() {	// (11)framing error
                let err = b"\r\nDetected framing error.\r\n";
                let _ = dp.USART2.dr.read().bits();
                for c in err.iter() {
                    let data: u32 = *c as u32;
                    dp.USART2.dr.write( |w| unsafe { w.bits(data) });
                    while !dp.USART2.sr.read().txe().bit() {}
                }
            }
            else if dp.USART2.sr.read().ore().bit() {	// (12)overrun error
                let err = b"\r\nDetected overrun error.\r\n";
                let _ = dp.USART2.dr.read().bits();
                for c in err.iter() {
                    let data: u32 = *c as u32;
                    dp.USART2.dr.write( |w| unsafe { w.bits(data) });
                    while !dp.USART2.sr.read().txe().bit() {}
                }
            }
            else {	// (13)no error
                dp.USART2.dr.write( |w| unsafe { w.bits(dp.USART2.dr.read().bits()) });	// echo back
                while !dp.USART2.sr.read().txe().bit() {}
            }
        }
    }
}

fn clock_init(dp: &stm32f401::Peripherals) {

    // PLLSRC = HSI: 16MHz (default)
    dp.RCC.pllcfgr.modify(|_, w| w.pllp().div4());      // (14)P=4
    dp.RCC.pllcfgr.modify(|_, w| unsafe { w.plln().bits(336) });    // (15)N=336
    // PLLM = 16 (default)

    dp.RCC.cfgr.modify(|_, w| w.ppre1().div2());        // (16) APB1 PSC = 1/2
    dp.RCC.cr.modify(|_, w| w.pllon().on());            // (17)PLL On
    while dp.RCC.cr.read().pllrdy().is_not_ready() {    // (18)安定するまで待つ
        // PLLがロックするまで待つ (PLLRDY)
    }

    // データシートのテーブル15より
    dp.FLASH.acr.modify(|_,w| w.latency().bits(2));    // (19)レイテンシの設定: 2ウェイト

    dp.RCC.cfgr.modify(|_,w| w.sw().pll());     // (20)sysclk = PLL
    while !dp.RCC.cfgr.read().sws().is_pll() {  // (21)SWS システムクロックソースがPLLになるまで待つ
    }

//  SYSCLK = 16MHz * 1/M * N * 1/P
//  SYSCLK = 16MHz * 1/16 * 336 * 1/4 = 84MHz
//  APB1 = 42MHz (USTAR2 pclk1)

}

fn gpioa2a3_init(dp: &stm32f401::Peripherals) {
    dp.RCC.ahb1enr.modify(|_, w| w.gpioaen().enabled());    // (22)GPIOAのクロックを有効にする
    dp.GPIOA.moder.modify(|_, w| w.moder2().alternate());   // (23)GPIOA2をオルタネートに設定    
    dp.GPIOA.moder.modify(|_, w| w.moder3().alternate());   // (24)GPIOA3をオルタネートに設定    
    dp.GPIOA.afrl.modify(|_, w| w.afrl2().af7());           // (25)GPIOA2をAF7に設定    
    dp.GPIOA.afrl.modify(|_, w| w.afrl3().af7());           // (26)GPIOA3をAF7に設定
    
    // GPIOA2 = USART2 Tx
    // GPIOA3 = USART2 Rx
}

fn usart2_init(dp: &stm32f401::Peripherals) {

    // 通信速度: 115,200
    // データ長: 7ビット
    // パリティ: 偶数(EVEN)
    // ストップビット: 1ビット

    dp.RCC.apb1enr.modify(|_,w| w.usart2en().enabled());    // (27)USART2のクロックを有効にする
    dp.USART2.cr1.modify(|_, w| w.te().enabled());  // (28)送信有効
    dp.USART2.cr1.modify(|_, w| w.re().enabled());  // (29)受信有効
    dp.USART2.cr1.modify(|_, w| w.pce().enabled()); // (30)パリティチェック有効
    dp.USART2.cr1.modify(|_, w| w.ue().enabled());  // (31)USART有効

// 以下のようにまとめて書くこともできる
// dp.USART2.cr1.modify(|_, w| w.te().enabled().re().enabled().pce().enabled().ue().enabled());

    dp.USART2.brr.modify(|_, w| w.div_mantissa().bits(22)); // (32)ボーレート（整数部）
    dp.USART2.brr.modify(|_, w| w.div_fraction().bits(12)); // (33)ボーレート（小数部）

//  bps = pclk1 / 16 * USARTDIV
//  USARTDIV = 22 + 12/16 = 22.75
//  bps = 42M / 16 * 22.75 = 42M / 364 =115384
//  誤差 115384/115200 = 1.001597222
}

