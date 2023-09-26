pub mod config;
pub mod orderbook;
pub mod web;

pub trait Exchange {}

impl Exchange for () {}
