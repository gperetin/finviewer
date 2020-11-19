use std::sync::Arc;
use chrono::naive::NaiveDate;

use druid::{AppLauncher, Color, Data, Lens, Rect, Widget, WindowDesc, PlatformError};

use druid::widget::{Flex, Label, Padding};
use druid::widget::prelude::*;


#[derive(Clone, Debug, Lens, Data)]
struct Bar {
    date: Arc<NaiveDate>, // wrap this is Arc because Data trait is implemented for that.
    open: f64,
    high: f64,
    low: f64,
    close: f64
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

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppData, env: &Env) {
        // here we iterate over all the bars in AppData and draw a rectangle for each
        // we should also have some spacing between bars
        // Challenge:
        // - translation between prices of the bar into coordinates on the screen
        // How to figure out Y axis?
        // - while drawing, remember min and max value drawn, then add some padding on each
        //
        // ctx.size() returns the size of the rectangle we're painting into
        // somehow we have to map bars onto that.
        // - one solution is to do 2 passes, compute difference between min and max price
        //   and divide that by the vertical height of the size - some_padding, so we have mapping
        //   from price onto pixels
        //
        // here's how to draw a rectangle
        // from_origin_size means it starts at (10,10) and is 100 wide and 100 tall
        let rect = Rect::from_origin_size((10.0, 10.0), (100.0, 100.0));
        // Note the Color:rgba8 which includes an alpha channel (7F in this case)
        let fill_color = Color::rgba8(0x00, 0x00, 0x00, 0x7F);

        // I think this call should do the actual painting, but need to verify that.
        ctx.fill(rect, &fill_color);
    }
}


fn build_ui() -> impl Widget<()> {
    Padding::new(5.0,
        Flex::row()
            .with_flex_child(
                Flex::column()
                    .with_flex_child(Label::new("Symbol: "), 1.0)
                    .with_child(Label::new("Graph here ")),
                1.0)
    )
}

// TODO:
// - make a chart widget that shows dummy data

fn main() -> Result<(), PlatformError> {
    AppLauncher::with_window(WindowDesc::new(build_ui)).launch(())?;
    Ok(())
}
