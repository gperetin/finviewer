use std::time::Instant;

use crate::Chart;

use chrono::Datelike;

use druid::{Color, Rect};
use druid::piet::{FontFamily, RenderContext, Text, TextLayout, TextLayoutBuilder};
use druid::kurbo::Line;
use druid::widget::prelude::*;

const BAR_WIDTH: i32 = 10;
const BAR_SPACING: i32 = 5;
const X_AXIS_LABELS_PADDING: f64 = 20.0;
const Y_TICK_SPACING: f64 = 50.0; // Ticks on y axis every 50 pixels
const Y_AXIS_LABELS_PADDING: f64 = 40.0;
static Y_AXIS_TICK_INCREMENTS: &'static [f64] = &[0.1, 0.5, 1.0, 10.0, 100.0];
const TEXT_COLOR: Color = Color::rgb8(0xef, 0xf8, 0xff);

pub struct ChartWidget;

impl Widget<Chart> for ChartWidget {

    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut Chart, _env: &Env) {}

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &Chart,
        _env: &Env,
    ) {
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &Chart, _data: &Chart, _env: &Env) {}

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &Chart,
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

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Chart, _env: &Env) {
        let start_time = Instant::now();

        let size = ctx.size();
        // size is 100x100 because that's what we defined in the layout method()

        // TODO: extract this into a method on Chart, price_range() or something
        let mut max_price: f64 = data.bars.first().unwrap().high;
        let mut min_price: f64 = data.bars.first().unwrap().low;

        let mut bars_to_render = vec![];

        for bar in data.bars.iter() {
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
        ctx.stroke(x_axis, &TEXT_COLOR, 1.0);
        ctx.stroke(y_axis, &TEXT_COLOR, 1.0);

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

            ctx.stroke(tick_line, &TEXT_COLOR, 1.0);

            // Put the tick label
            let price_label = max_price - current_y_tick;
            let layout = ctx
                .text()
                .new_text_layout(price_label.to_string())
                .font(FontFamily::SANS_SERIF, 14.0)
                .text_color(TEXT_COLOR)
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
            ctx.stroke(wick, &TEXT_COLOR, 1.0);

            // Now let's plot the candle body
            let higher_value = if bar.close > bar.open { bar.close } else { bar.open };
            let bar_y_top: f64 = (max_price - higher_value) * scaling;

            let bar_start = ((x_position - BAR_WIDTH/2) as f64, bar_y_top);

            let lower_value = if bar.close > bar.open { bar.open } else { bar.close };
            let bar_height = ((max_price - lower_value) * scaling) - bar_y_top;

            // from_origin_size means it starts at (10,10) and is 100 wide and 100 tall
            let bar_rect = Rect::from_origin_size(bar_start, (BAR_WIDTH as f64, bar_height as f64));

            let fill_color = if higher_value == bar.close {
                Color::rgb8(0x38, 0xc1, 0x72)
                // let fill_color = Color::rgba8(0x00, 0x00, 0x00, 0x7F);
            } else {
                Color::rgb8(0xdc, 0x30, 0x30)
            };

            ctx.fill(bar_rect, &fill_color);

            // Put X-axis label
            let layout = ctx
                .text()
                .new_text_layout(bar.date.day().to_string())
                .font(FontFamily::SANS_SERIF, 14.0)
                .text_color(TEXT_COLOR)
                .build()
                .unwrap();
            ctx.draw_text(&layout, (x_position as f64, size.height - X_AXIS_LABELS_PADDING + 5.0));

            x_position += BAR_WIDTH + BAR_SPACING;
        }

        println!("Total render time: {:?} milliseconds", start_time.elapsed().as_millis());
    }
}

