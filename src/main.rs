extern crate config;

use std::sync::Arc;
use std::time::Instant;
use chrono::{Datelike, NaiveDateTime};

mod lib;
use lib::Bar;

use druid::{AppLauncher, Color, Data, Lens, Rect, Widget, WindowDesc, PlatformError};
use druid::kurbo::Line;
use druid::piet::{FontFamily, RenderContext, Text, TextLayout, TextLayoutBuilder};

use druid::widget::{Flex, Label, Padding, SizedBox};
use druid::widget::prelude::*;

use postgres::{Client, NoTls};

use rust_decimal::prelude::*;


const BAR_WIDTH: i32 = 10;
const BAR_SPACING: i32 = 5;
const X_AXIS_LABELS_PADDING: f64 = 20.0;
const Y_TICK_SPACING: f64 = 50.0; // Ticks on y axis every 50 pixels
const Y_AXIS_LABELS_PADDING: f64 = 40.0;
static Y_AXIS_TICK_INCREMENTS: &'static [f64] = &[0.1, 0.5, 1.0, 10.0, 100.0];


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
    chart: Arc<Vec<Bar>>
}


struct ChartWidget;

impl Widget<AppData> for ChartWidget {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut AppData, _env: &Env) {}

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &AppData,
        _env: &Env,
    ) {
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &AppData, _data: &AppData, _env: &Env) {}

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &AppData,
        _env: &Env,
    ) -> Size {
        // BoxConstraints are passed by the parent widget.
        // This method can return any Size within those constraints:
        // bc.constrain(my_size)
        //
        // To check if a dimension is infinite or not (e.g. scrolling):
        // bc.is_width_bounded() / bc.is_height_bounded()
        //
        // bx.max() returns the maximum size of the widget. Be careful
        // using this, since always make sure the widget is bounded.
        // If bx.max() is used in a scrolling widget things will probably
        // not work correctly.
        if bc.is_width_bounded() | bc.is_height_bounded() {
            let size = Size::new(100.0, 100.0);
            bc.constrain(size)
        } else {
            bc.max()
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppData, _env: &Env) {
        let start_time = Instant::now();

        let size = ctx.size();
        // size is 100x100 because that's what we defined in the layout method()

        let mut max_price: f64 = data.chart.first().unwrap().high;
        let mut min_price: f64 = data.chart.first().unwrap().low;

        let mut bars_to_render = vec![];

        for bar in data.chart.iter() {
            // Let's only plot bars that we can fit into the available screen area
            if bars_to_render.len() as i32 * (BAR_WIDTH + BAR_SPACING) > (size.width - Y_AXIS_LABELS_PADDING - 2.0 * BAR_SPACING as f64) as i32 {
                break;
            }

            if bar.high > max_price {
                max_price = bar.high
            }
            if bar.low < min_price {
                min_price = bar.low
            }
            bars_to_render.push(bar);
        }

        let scaling: f64 = (size.height - X_AXIS_LABELS_PADDING) / (max_price - min_price);

        // Plot axis
        let x_axis = Line::new(
            (BAR_SPACING as f64, size.height - X_AXIS_LABELS_PADDING),
            (size.width - Y_AXIS_LABELS_PADDING, size.height - X_AXIS_LABELS_PADDING)
        );
        let y_axis = Line::new(
            (size.width - Y_AXIS_LABELS_PADDING, BAR_SPACING as f64),
            (size.width - Y_AXIS_LABELS_PADDING, size.height - BAR_SPACING as f64)
        );
        ctx.stroke(x_axis, &Color::WHITE, 1.0);
        ctx.stroke(y_axis, &Color::WHITE, 1.0);

        // Plot ticks on Y axis
        let price_range = max_price - min_price;
        let approx_num_of_ticks = size.height / Y_TICK_SPACING;
        let mut closest_tick_size = Y_AXIS_TICK_INCREMENTS[0];
        let mut closest_num_ticks = price_range / closest_tick_size;

        for tick_size in Y_AXIS_TICK_INCREMENTS {
            if ((price_range / tick_size) - approx_num_of_ticks).abs() < (closest_num_ticks - approx_num_of_ticks).abs() {
                closest_tick_size = *tick_size;
                closest_num_ticks = price_range / *tick_size;
            }
        }

        let y_tick_start = max_price % closest_tick_size;
        let mut current_y_tick = y_tick_start;
        while (max_price - current_y_tick) > min_price {
            let tick_line = Line::new(
                (size.width - Y_AXIS_LABELS_PADDING, current_y_tick * scaling),
                (size.width - Y_AXIS_LABELS_PADDING + 5.0, current_y_tick * scaling)
            );

            ctx.stroke(tick_line, &Color::WHITE, 1.0);

            // Put the tick label
            let price_label = max_price - current_y_tick;
            let layout = ctx
                .text()
                .new_text_layout(price_label.to_string())
                .font(FontFamily::SERIF, 12.0)
                .text_color(Color::WHITE)
                .build()
                .unwrap();
            ctx.draw_text(
                &layout,
                (size.width - Y_AXIS_LABELS_PADDING + 10.0, current_y_tick * scaling - layout.size().height / 2.0)
            );

            current_y_tick += closest_tick_size;
        }

        // Plot candlesticks
        let mut x_position: i32 = BAR_SPACING * 2; // Let's leave some padding to the left

        bars_to_render.reverse();
        for bar in bars_to_render {
            // Let's plot the wick first
            let bar_high = (max_price - bar.high) * scaling;
            let bar_low = (max_price - bar.low) * scaling;

            let wick = Line::new((x_position as f64, bar_high as f64), (x_position as f64, bar_low as f64));
            ctx.stroke(wick, &Color::rgb8(105, 105, 105), 1.0);

            // Now let's plot the candle body
            let higher_value = if bar.close > bar.open { bar.close } else { bar.open };
            let bar_y_top: f64 = (max_price - higher_value) * scaling;

            let bar_start = ((x_position - BAR_WIDTH/2) as f64, bar_y_top);

            let lower_value = if bar.close > bar.open { bar.open } else { bar.close };
            let bar_height = ((max_price - lower_value) * scaling) - bar_y_top;

            // from_origin_size means it starts at (10,10) and is 100 wide and 100 tall
            let bar_rect = Rect::from_origin_size(bar_start, (BAR_WIDTH as f64, bar_height as f64));

            let fill_color = if higher_value == bar.close {
                Color::rgb(0x00, 0xFF, 0x00)
                // let fill_color = Color::rgba8(0x00, 0x00, 0x00, 0x7F);
            } else {
                Color::rgb(0xFF, 0x00, 0x00)
            };

            ctx.fill(bar_rect, &fill_color);

            // Put X-axis label
            let layout = ctx
                .text()
                .new_text_layout(bar.date.day().to_string())
                .font(FontFamily::SERIF, 12.0)
                .text_color(Color::WHITE)
                .build()
                .unwrap();
            ctx.draw_text(&layout, (x_position as f64, size.height - X_AXIS_LABELS_PADDING + 5.0));

            x_position += BAR_WIDTH + BAR_SPACING;
        }

        println!("Total render time: {:?} milliseconds", start_time.elapsed().as_millis());
    }
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
                    .with_flex_child(SizedBox::new(Padding::new(10.0, ChartWidget {})).expand_height().expand_width(), 5.0),
                1.0)
    )
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
    launcher.launch(AppData { chart: bars })?;
    Ok(())
}
