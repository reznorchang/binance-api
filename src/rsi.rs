use crate::market::*;
use crate::model::*;
use crate::api::*;
use crate::account::*;
use crate::config::*;

pub fn rsi_order(symbol: &str, window_size: usize, use_testnet: bool) {
    //grant account/market access
    let api_key = Some("".into());
    let secret_key = Some("".into());

    let account: Account = if use_testnet {
        let config = Config::default().set_rest_api_endpoint("https://testnet.binance.vision");
        Binance::new_with_config(api_key, secret_key, &config)
    } else {
        Binance::new(api_key, secret_key)
    };

    let market: Market = Binance::new(None, None);
    //get limit price of symbol
    let mut limit_price: f64 = 0.0;
    match market.get_price(symbol) {
        Ok(answer) => {
            limit_price = answer.price;
            println!("{:?}", answer)
        }
        Err(e) => println!("Error: {}", e),
    }
    //calc rsi for the symbol and submit order
    let cur_rsi = rsi_calc_1d(market, symbol, window_size);
    match cur_rsi {
        0..=19 => match account.limit_buy(symbol, 0.001, limit_price) {
            Ok(answer) => println!(
                "{} days frame RSI value:{} => RSI below lo limit, buy order submitted\n{:?}",
                window_size, cur_rsi, answer
            ),
            Err(e) => println!("Error: {}", e),
        },
        20..=60 => println!(
            "{} days frame RSI value:{} : RSI within hi-lo limit, no order submitted",
            window_size, cur_rsi
        ),
        61..=100 => match account.limit_sell(symbol, 0.001, limit_price) {
            Ok(answer) => println!(
                "{} days frame RSI value:{} => RSI higher hi limit, sell order submitted\n{:?}",
                window_size, cur_rsi, answer
            ),
            Err(e) => println!("Error: {}", e),
        },
        _ => panic!(),
    }
}

fn rsi_calc_1d(market: Market, symbol: &str, window_size: usize) -> i32 {
    //get close prize of size+1 days
    let mut price_series = Vec::new();
    match market.get_klines(symbol, "1d", window_size as u16 + 1, None, None) {
        Ok(ksums) => match ksums {
            KlineSummaries::AllKlineSummaries(klines) => {
                klines
                    .iter()
                    .for_each(|it| price_series.push(it.close.parse::<f64>().unwrap()));
            }
        },
        Err(e) => println!("Error: {}", e),
    }
    //calculate rsi and return
    let rsi_series = relative_strength_index(&price_series, window_size).unwrap();
    //check value rationality

    //return value
    rsi_series[0].round() as i32
}

fn relative_strength_index(data_set: &Vec<f64>, window_size: usize) -> Option<Vec<f64>> {
    let mut result: Vec<f64> = Vec::new();
    if window_size > data_set.len() {
        return None;
    }
    let mut previous_average_gain;
    let mut previous_average_loss;
    // RSI Step one
    let mut gains_sum = 0.0;
    let mut loss_sum = 0.0;
    for i in 0..(window_size + 1) {
        let gain = if i == 0 {
            0.0
        } else {
            (100.0 / data_set[i - 1]) * data_set[i] - 100.0
        };
        if gain >= 0.0 {
            gains_sum += gain;
        } else {
            loss_sum += gain.abs();
        }
    }
    let current_average_gain = gains_sum / window_size as f64;
    let current_average_loss = loss_sum / window_size as f64;
    let rsi_a = 100.0 - 100.0 / (1.0 + (current_average_gain / current_average_loss));
    previous_average_gain = current_average_gain;
    previous_average_loss = current_average_loss;
    result.push(rsi_a);

    // RSI Step two
    for i in (window_size + 1)..data_set.len() {
        let gain = (100.0 / data_set[i - 1]) * data_set[i] - 100.0;
        let (current_gain, current_loss) = if gain > 0.0 {
            (gain, 0.0)
        } else {
            (0.0, gain.abs())
        };
        let current_average_gain = (previous_average_gain * (window_size as f64 - 1.0)
            + current_gain)
            / window_size as f64;
        let current_average_loss = (previous_average_loss * (window_size as f64 - 1.0)
            + current_loss)
            / window_size as f64;
        previous_average_gain = current_average_gain;
        previous_average_loss = current_average_loss;
        let rsi = 100.0 - 100.0 / (1.0 + current_average_gain / current_average_loss);
        result.push(rsi);
    }
    Some(result)
}
