use std::{time::Instant, vec};
use std::sync::Arc;

use crate::{Bar, Chart};

use chrono::Datelike;

use druid::{Color, Rect};
use druid::piet::{FontFamily, RenderContext, Text, TextLayout, TextLayoutBuilder};
use druid::kurbo::Line;
use druid::widget::prelude::*;

const FONT_SIZE: f64 = 14.0;
const BAR_WIDTH: i32 = 10;
const BAR_SPACING: i32 = 5;
const X_AXIS_LABELS_PADDING: f64 = 20.0;
const Y_TICK_SPACING: f64 = 50.0; // Ticks on y axis every 50 pixels
const Y_AXIS_LABELS_PADDING: f64 = 40.0;
static Y_AXIS_TICK_INCREMENTS: &'static [f64] = &[0.1, 0.5, 1.0, 10.0, 100.0];
const TEXT_COLOR: Color = Color::rgb8(0xef, 0xf8, 0xff);

struct PriceRange {
    lowest: f64,
    highest: f64
}

impl PriceRange {
    fn range(&self) -> f64 {
        self.highest - self.lowest
    }
}

pub struct ChartWidget {
    bars: Arc<Vec<Bar>>,
    size: Size
}

struct AxisLabel {
    /// Label text
    label: String,
    
    /// Position where to draw the label
    position: (f64, f64)
}

struct AxisTick {
    /// How many pixels from the start (0) in should we place the tick
    tick_line: Line,

    /// What is the label we should print with the tick
    label: AxisLabel
}

struct Candle {
    wick: Line,
    body: Rect,
    color: Color
}

impl ChartWidget {
    pub fn new(chart: &Chart, size: Size) -> Self {
        Self {
            bars: chart.clone().bars,
            size: size
        }
    }

    pub fn empty() -> Self {
        Self {
            bars: Arc::new(vec![]),
            size: Size::new(0.0, 0.0)
        }
    }

    /// The price range of the bars that will be visible on the rendered graph
    fn price_range(&self, bars: Vec<&Bar>) -> PriceRange {
        // TODO: let's try to figure out how to merge this pass with the one below in
        // `visible_bars` so we don't do 2 passes
        let mut max_price: f64 = self.bars.first().unwrap().high;
        let mut min_price: f64 = self.bars.first().unwrap().low;

        for bar in bars.iter() {
            if bar.high > max_price {
                max_price = bar.high
            }
            if bar.low < min_price {
                min_price = bar.low
            }
        }
        PriceRange { lowest: min_price, highest: max_price }
    }

    /// Returns a list of bars that will be visible on the chart given the chart
    /// size and padding settings
    fn visible_bars(&self) -> Vec<&Bar> {
        let mut bars_to_render: Vec<&Bar> = vec![];

        // TODO: implement rendering charts that don't have the last price on the right edge, but
        // have a custom range.
        for bar in self.bars.iter() {
            // Let's only plot bars that we can fit into the available screen area
            if bars_to_render.len() as i32 * (BAR_WIDTH + BAR_SPACING) > (self.size.width - Y_AXIS_LABELS_PADDING - 2.0 * BAR_SPACING as f64) as i32 {
                break;
            }
            bars_to_render.push(bar);
        }

        println!("Rendering bars from {:?} to {:?}", bars_to_render.first(), bars_to_render.last());

        bars_to_render.reverse();
        bars_to_render
    }

    fn x_axis(&self) -> Line {
        Line::new(
            (BAR_SPACING as f64, self.size.height - X_AXIS_LABELS_PADDING),
            (self.size.width - Y_AXIS_LABELS_PADDING, self.size.height - X_AXIS_LABELS_PADDING)
        )
    }

    fn y_axis(&self) -> Line {
        Line::new(
            (self.size.width - Y_AXIS_LABELS_PADDING, BAR_SPACING as f64),
            (self.size.width - Y_AXIS_LABELS_PADDING, self.size.height - BAR_SPACING as f64)
        )
    }

    fn y_axis_ticks(&self) -> Vec<AxisTick> {
        let price_range = self.price_range(self.visible_bars());
        let approx_num_of_ticks = self.size.height / Y_TICK_SPACING;
        let mut closest_tick_size = Y_AXIS_TICK_INCREMENTS[0];
        let mut closest_num_ticks = price_range.range() / closest_tick_size;

        for tick_size in Y_AXIS_TICK_INCREMENTS {
            if ((price_range.range() / tick_size) - approx_num_of_ticks).abs() < (closest_num_ticks - approx_num_of_ticks).abs() {
                closest_tick_size = *tick_size;
                closest_num_ticks = price_range.range() / *tick_size;
            }
        }

        let scaling: f64 = (self.size.height - X_AXIS_LABELS_PADDING) / price_range.range();
        let y_tick_start = price_range.highest % closest_tick_size;
        let mut current_y_tick = y_tick_start;
        let mut ticks: Vec<AxisTick> = vec![];
        while (price_range.highest - current_y_tick) > price_range.lowest {
            let price_label = price_range.highest - current_y_tick;
            let pixel_offset = current_y_tick * scaling;
            let tick_line = Line::new(
                (self.size.width - Y_AXIS_LABELS_PADDING, pixel_offset),
                (self.size.width - Y_AXIS_LABELS_PADDING + 5.0, pixel_offset)
            );

            ticks.push(AxisTick {
                tick_line: tick_line,
                label: AxisLabel {
                    label: price_label.to_string(),
                    position: (self.size.width - Y_AXIS_LABELS_PADDING + 10.0, pixel_offset - FONT_SIZE / 2.0)
                }
            } );

            current_y_tick += closest_tick_size;
        }

        ticks
    }

    /// Returns candles to be plotted
    fn candles(&self) -> Vec<Candle> {
        // Now let's plot the candle body
        let price_range = self.price_range(self.visible_bars());
        let scaling: f64 = (self.size.height - X_AXIS_LABELS_PADDING) / price_range.range();
        let mut x_position: i32 = BAR_SPACING * 2; // Let's leave some padding to the left
        let mut candles: Vec<Candle> = vec![];

        for bar in self.visible_bars() {
            // Wick
            let bar_high = (price_range.highest - bar.high) * scaling;
            let bar_low = (price_range.highest - bar.low) * scaling;
            let wick = Line::new((x_position as f64, bar_high as f64), (x_position as f64, bar_low as f64));

            // Candle body
            let higher_value = if bar.close > bar.open { bar.close } else { bar.open };
            let bar_y_top: f64 = (price_range.highest - higher_value) * scaling;

            let bar_start = ((x_position - BAR_WIDTH/2) as f64, bar_y_top);

            let lower_value = if bar.close > bar.open { bar.open } else { bar.close };
            let bar_height = ((price_range.highest - lower_value) * scaling) - bar_y_top;

            // from_origin_size means it starts at (10,10) and is 100 wide and 100 tall
            let bar_rect = Rect::from_origin_size(bar_start, (BAR_WIDTH as f64, bar_height as f64));

            // Color
            let fill_color = if higher_value == bar.close {
                Color::rgb8(0x38, 0xc1, 0x72)
                // let fill_color = Color::rgba8(0x00, 0x00, 0x00, 0x7F);
            } else {
                Color::rgb8(0xdc, 0x30, 0x30)
            };

            x_position += BAR_WIDTH + BAR_SPACING;

            candles.push(Candle {
                wick: wick,
                body: bar_rect,
                color: fill_color
            });
        }

        candles
    }
}

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

        let widget = ChartWidget::new(data, ctx.size());
        ctx.stroke(widget.x_axis(), &TEXT_COLOR, 1.0);
        ctx.stroke(widget.y_axis(), &TEXT_COLOR, 1.0);

        // Plot ticks on Y axis
        for tick in widget.y_axis_ticks() {
            ctx.stroke(tick.tick_line, &TEXT_COLOR, 1.0);

            let layout = ctx
                .text()
                .new_text_layout(tick.label.label)
                .font(FontFamily::SANS_SERIF, FONT_SIZE)
                .text_color(TEXT_COLOR)
                .build()
                .unwrap();
            ctx.draw_text(&layout, tick.label.position);
        }

        for candle in widget.candles() {
            ctx.stroke(candle.wick, &TEXT_COLOR, 1.0);

            // TODO: plot the x-axis label
            // let layout = ctx
            //     .text()
            //     .new_text_layout(bar.date.day().to_string())
            //     .font(FontFamily::SANS_SERIF, 14.0)
            //     .text_color(TEXT_COLOR)
            //     .build()
            //     .unwrap();
            // ctx.draw_text(&layout, (x_position as f64, size.height - X_AXIS_LABELS_PADDING + 5.0));

            ctx.fill(candle.body, &candle.color);
        }

        println!("Total render time: {:?} milliseconds", start_time.elapsed().as_millis());
    }
}
