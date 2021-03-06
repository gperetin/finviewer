use std::sync::Arc;

use druid::{Data, Lens};
use chrono::NaiveDate;


#[derive(Clone, Debug, Lens, Data)]
pub struct Bar {
    pub date: Arc<NaiveDate>, // wrap this is Arc because Data trait is implemented for that.
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64
}


#[derive(Debug, Clone, Data, Lens)]
pub struct Chart {
    pub bars: Arc<Vec<Bar>>
}

impl Chart {
    pub fn new() -> Self {
        Self {
            bars: Arc::new(vec![])
        }
    }
}
