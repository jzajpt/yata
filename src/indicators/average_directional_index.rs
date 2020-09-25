#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::core::{IndicatorConfig, IndicatorInitializer, IndicatorInstance, IndicatorResult};
use crate::core::{PeriodType, ValueType, Window, OHLC};
use crate::helpers::{method, RegularMethod, RegularMethods};

/// [Average Directional Index](https://www.investopedia.com/terms/a/adx.asp)
///
/// ## Links:
///
/// * https://school.stockcharts.com/doku.php?id=technical_indicators:average_directional_index_adx
/// * https://www.investopedia.com/terms/a/adx.asp
/// * https://primexbt.com/blog/average-directional-index/
///
/// # 3 values
/// * ADX
/// * +DI
/// * -DI
///
/// # 2 signals
/// * `BUY_ALL` when ADX over `zone` and +DI > -DI, `SELL_ALL` when ADX over `zone` and -DI > +DI. Otherwise - no signal.
/// * Digital signal by difference between +DI and -DI
///
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AverageDirectionalIndex {
	pub method1: RegularMethods,
	pub di_length: PeriodType,

	pub method2: RegularMethods,
	pub adx_smoothing: PeriodType,

	pub period1: PeriodType,
	pub zone: ValueType,
}

impl IndicatorConfig for AverageDirectionalIndex {
	fn validate(&self) -> bool {
		self.di_length >= 1
			&& self.adx_smoothing >= 1
			&& self.zone >= 0.
			&& self.zone <= 1.
			&& self.period1 >= 1
			&& self.period1 < self.di_length
			&& self.period1 < self.adx_smoothing
	}

	fn set(&mut self, name: &str, value: String) {
		match name {
			"method1" => self.method1 = value.parse().unwrap(),
			"di_length" => self.di_length = value.parse().unwrap(),

			"method2" => self.method2 = value.parse().unwrap(),
			"adx_smoothing" => self.adx_smoothing = value.parse().unwrap(),

			"period1" => self.period1 = value.parse().unwrap(),
			"zone" => self.zone = value.parse().unwrap(),

			_ => {
				dbg!(format!(
					"Unknown attribute `{:}` with value `{:}` for `{:}`",
					name,
					value,
					std::any::type_name::<Self>(),
				));
			}
		};
	}

	fn size(&self) -> (u8, u8) {
		(3, 2)
	}
}

impl<T: OHLC> IndicatorInitializer<T> for AverageDirectionalIndex {
	type Instance = AverageDirectionalIndexInstance<T>;
	fn init(self, candle: T) -> Self::Instance
	where
		Self: Sized,
	{
		let cfg = self;
		let tr = candle.tr(&candle);

		Self::Instance {
			window: Window::new(cfg.period1, candle),
			tr_ma: method(cfg.method1, cfg.di_length, tr),
			plus_di: method(cfg.method1, cfg.di_length, 0.0),
			minus_di: method(cfg.method1, cfg.di_length, 0.0),
			ma2: method(cfg.method2, cfg.adx_smoothing, 0.0),
			cfg,
		}
	}
}

impl Default for AverageDirectionalIndex {
	fn default() -> Self {
		Self {
			method1: RegularMethods::RMA,
			di_length: 14,
			method2: RegularMethods::RMA,
			adx_smoothing: 14,
			period1: 1,
			zone: 0.2,
		}
	}
}

#[derive(Debug)]
pub struct AverageDirectionalIndexInstance<T: OHLC> {
	cfg: AverageDirectionalIndex,

	window: Window<T>,
	tr_ma: RegularMethod,
	plus_di: RegularMethod,
	minus_di: RegularMethod,
	ma2: RegularMethod,
}

impl<T: OHLC> AverageDirectionalIndexInstance<T> {
	fn dir_mov(&mut self, candle: T) -> (ValueType, ValueType) {
		let prev_candle = self.window.push(candle);
		let true_range = self.tr_ma.next(candle.tr(&prev_candle));

		let (du, dd) = (
			candle.high() - prev_candle.high(),
			prev_candle.low() - candle.low(),
		);

		let plus_dm = du * (du > dd && du > 0.) as u8 as ValueType; // +DM
		let minus_dm = dd * (dd > du && dd > 0.) as u8 as ValueType; // -DM

		let plus_di_value = self.plus_di.next(plus_dm); // +DI
		let minus_di_value = self.minus_di.next(minus_dm); // -DI

		(plus_di_value / true_range, minus_di_value / true_range)
	}

	fn adx(&mut self, plus: ValueType, minus: ValueType) -> ValueType {
		let s = plus + minus;

		if s == 0. {
			return self.ma2.next(0.);
		}

		let t = (plus - minus).abs() / s;
		self.ma2.next(t)
	}
}

impl<T: OHLC> IndicatorInstance<T> for AverageDirectionalIndexInstance<T> {
	type Config = AverageDirectionalIndex;

	fn config(&self) -> &Self::Config {
		&self.cfg
	}

	fn next(&mut self, candle: T) -> IndicatorResult {
		let (plus, minus) = self.dir_mov(candle);
		let adx = self.adx(plus, minus);

		let signal1 = (adx > self.cfg.zone) as i8 * ((plus > minus) as i8 - (plus < minus) as i8);
		let signal2 = plus - minus;

		let values = [adx, plus, minus];
		let signals = [signal1.into(), signal2.into()];

		IndicatorResult::new(&values, &signals)
	}
}
