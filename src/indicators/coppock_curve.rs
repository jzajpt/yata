#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::core::{IndicatorConfig, IndicatorInitializer, IndicatorInstance, IndicatorResult};
use crate::core::{Method, PeriodType, Source, OHLC};
use crate::helpers::{method, RegularMethod, RegularMethods};
use crate::methods::{Cross, RateOfChange, ReverseSignal};

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoppockCurve {
	pub period1: PeriodType,
	pub period2: PeriodType,
	pub period3: PeriodType,
	pub s2_left: PeriodType,
	pub s2_right: PeriodType,
	pub s3_period: PeriodType,
	pub source: Source,
	pub method1: RegularMethods,
	pub method2: RegularMethods,
}

impl IndicatorConfig for CoppockCurve {
	fn validate(&self) -> bool {
		true
	}

	fn set(&mut self, name: &str, value: String) {
		match name {
			"period1" => self.period1 = value.parse().unwrap(),
			"period2" => self.period2 = value.parse().unwrap(),
			"period3" => self.period3 = value.parse().unwrap(),
			"s2_left" => self.s2_left = value.parse().unwrap(),
			"s2_right" => self.s2_right = value.parse().unwrap(),
			"s3_period" => self.s3_period = value.parse().unwrap(),
			"source" => self.source = value.parse().unwrap(),
			"method1" => self.method1 = value.parse().unwrap(),
			"method2" => self.method2 = value.parse().unwrap(),
			// "zone"		=> self.zone = value.parse().unwrap(),
			// "source"	=> self.source = value.parse().unwrap(),
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
		(2, 3)
	}
}

impl<T: OHLC> IndicatorInitializer<T> for CoppockCurve {
	type Instance = CoppockCurveInstance;

	fn init(self, candle: T) -> Self::Instance
	where
		Self: Sized,
	{
		let cfg = self;
		let src = candle.source(cfg.source);
		Self::Instance {
			roc1: RateOfChange::new(cfg.period2, src),
			roc2: RateOfChange::new(cfg.period3, src),
			ma1: method(cfg.method1, cfg.period1, 0.),
			ma2: method(cfg.method2, cfg.s3_period, 0.),
			cross_over1: Cross::default(),
			pivot: ReverseSignal::new(cfg.s2_left, cfg.s2_right, 0.),
			cross_over2: Cross::default(),

			cfg,
		}
	}
}

impl Default for CoppockCurve {
	fn default() -> Self {
		Self {
			period1: 10,
			period2: 14,
			period3: 11,
			s2_left: 4,
			s2_right: 2,
			s3_period: 5,
			method1: RegularMethods::WMA,
			method2: RegularMethods::EMA,
			source: Source::Close,
		}
	}
}

#[derive(Debug)]
pub struct CoppockCurveInstance {
	cfg: CoppockCurve,

	roc1: RateOfChange,
	roc2: RateOfChange,
	ma1: RegularMethod,
	ma2: RegularMethod,
	cross_over1: Cross,
	pivot: ReverseSignal,
	cross_over2: Cross,
}

impl<T: OHLC> IndicatorInstance<T> for CoppockCurveInstance {
	type Config = CoppockCurve;

	fn config(&self) -> &Self::Config {
		&self.cfg
	}

	fn next(&mut self, candle: T) -> IndicatorResult {
		let src = candle.source(self.cfg.source);
		let roc1 = self.roc1.next(src);
		let roc2 = self.roc2.next(src);
		let value1 = self.ma1.next(roc1 + roc2);
		let value2 = self.ma2.next(value1);

		let signal1 = self.cross_over1.next((value1, 0.));
		let signal2 = self.pivot.next(value1);
		let signal3 = self.cross_over2.next((value1, value2));

		IndicatorResult::new(&[value1, value2], &[signal1, signal2, signal3])
	}
}
