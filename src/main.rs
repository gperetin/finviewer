extern crate config;

use std::sync::Arc;
use chrono::NaiveDateTime;

mod types;
use types::{Bar, Chart};

mod chart_widget;
use chart_widget::ChartWidget;

use druid::{AppLauncher, Color, Data, Lens, Widget, WindowDesc, PlatformError};
use druid::widget::{Flex, Label, WidgetExt, Padding, SizedBox};
use postgres::{Client, NoTls};
use rust_decimal::prelude::*;

const DARK_GRAY: Color = Color::rgb8(0x21, 0x29, 0x34);


fn get_daily_price(mut client: Client, symbol: &str) -> Vec<Bar> {
    let mut bars = vec![];

    for row in client.query("
        SELECT * FROM myschema.adjusted_prices as ap JOIN myschema.instruments as instr
        ON ap.instrument_id = instr.id
        WHERE instr.symbol = $1
        ORDER BY ap.timestamp DESC LIMIT 100
    ", &[&symbol]).unwrap() {

        // NaiveDate::parse_from_str(row.get(1), "%Y-%m-%d %H:%M:%S").map(|date| {
        let dt = row.get::<usize, NaiveDateTime>(1);
        let bar = Bar {
            date: Arc::new(dt.date()),
            open: row.get::<usize, Decimal>(5).to_f64().unwrap(),
            high: row.get::<usize, Decimal>(6).to_f64().unwrap(),
            low: row.get::<usize, Decimal>(7).to_f64().unwrap(),
            close: row.get::<usize, Decimal>(4).to_f64().unwrap()
        };
        bars.push(bar);
    }
    bars
}

#[derive(Clone, Lens, Data)]
struct AppData {
    chart: Chart
}


fn build_ui() -> impl Widget<AppData> {
    // Check here to see how to make a complex layout
    // https://github.com/tbillington/kondo/blob/master/kondo-ui/src/main.rs#L239
    Padding::new(5.0,
        Flex::row()
            .with_flex_child(
                Flex::column()
                    .with_child(Label::new("Symbol: "))
                    .with_spacer(8.0)
                    .with_flex_child(
                        SizedBox::new(
                            Padding::new(10.0, (ChartWidget {}).lens(AppData::chart))
                        ).expand_height().expand_width(), 5.0),
                1.0)
    ).background(DARK_GRAY)
}

fn main() -> Result<(), PlatformError> {
    let window = WindowDesc::new(build_ui);
    let launcher = AppLauncher::with_window(window);

    let mut cfg = config::Config::default();
    cfg.merge(config::File::with_name("Config")).unwrap();

    // Instantiate the client and get the data
    let pg_client = Client::connect(
        format!("host={host} user={user} password={password} dbname={dbname}",
            host=&cfg.get_str("database.host").unwrap(),
            user=&cfg.get_str("database.user").unwrap(),
            password=&cfg.get_str("database.password").unwrap(),
            dbname=&cfg.get_str("database.dbname").unwrap(),
        ).as_str(), NoTls).unwrap();
    let bars = Arc::new(get_daily_price(pg_client, "ES"));
    launcher.launch(AppData { chart: Chart { bars }})?;
    Ok(())
}
