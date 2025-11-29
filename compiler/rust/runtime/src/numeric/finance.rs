#![cfg(feature = "decimal")]

use once_cell::sync::Lazy;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};

use crate::numeric::error::NumericError;
use crate::prelude::iter::Iter;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CurrencyCode {
    code: String,
}

impl CurrencyCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into().to_ascii_uppercase(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.code
    }
}

impl From<&str> for CurrencyCode {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for CurrencyCode {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

struct CurrencySpec {
    code: &'static str,
    scale: u8,
}

static CURRENCY_TABLE: Lazy<Vec<CurrencySpec>> = Lazy::new(|| {
    vec![
        CurrencySpec {
            code: "USD",
            scale: 2,
        },
        CurrencySpec {
            code: "EUR",
            scale: 2,
        },
        CurrencySpec {
            code: "JPY",
            scale: 0,
        },
        CurrencySpec {
            code: "GBP",
            scale: 2,
        },
    ]
});

fn lookup_currency(code: &CurrencyCode) -> Option<&'static CurrencySpec> {
    let upper = code.as_str();
    CURRENCY_TABLE
        .iter()
        .find(|spec| spec.code.eq_ignore_ascii_case(upper))
}

pub fn currency_add(
    a: Decimal,
    b: Decimal,
    currency: CurrencyCode,
) -> Result<Decimal, NumericError> {
    let spec = lookup_currency(&currency).ok_or_else(|| {
        NumericError::unsupported_currency("unsupported currency code")
            .with_currency_code(currency.as_str())
    })?;

    let result =
        (a + b).round_dp_with_strategy(spec.scale as u32, RoundingStrategy::MidpointNearestEven);
    Ok(result)
}

pub fn compound_interest(
    principal: Decimal,
    rate: f64,
    periods: u32,
) -> Result<Decimal, NumericError> {
    let rate_decimal = Decimal::from_f64(rate).ok_or_else(|| {
        NumericError::conversion_failed("invalid rate provided").with_value_repr(rate.to_string())
    })?;
    let factor = Decimal::ONE + rate_decimal;
    let result = if periods == 0 {
        principal
    } else {
        principal * pow_decimal(factor, periods)
    };
    Ok(result.normalize())
}

pub fn net_present_value(cashflows: Iter<Decimal>, rate: f64) -> Result<Decimal, NumericError> {
    let rate_decimal = Decimal::from_f64(rate).ok_or_else(|| {
        NumericError::conversion_failed("invalid discount rate provided")
            .with_value_repr(rate.to_string())
    })?;
    let factor = Decimal::ONE + rate_decimal;
    if factor == Decimal::ZERO {
        return Err(
            NumericError::invalid_precision("discount factor cannot be zero")
                .with_value_repr(rate.to_string()),
        );
    }

    let mut total = Decimal::ZERO;
    let mut discount = Decimal::ONE;
    for (index, amount) in cashflows.into_iter().enumerate() {
        if index > 0 {
            discount *= factor;
        }
        total += amount / discount;
    }
    Ok(total.normalize())
}

fn pow_decimal(base: Decimal, exp: u32) -> Decimal {
    let mut result = Decimal::ONE;
    for _ in 0..exp {
        result *= base;
    }
    result
}
