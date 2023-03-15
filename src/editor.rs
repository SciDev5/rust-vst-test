use atomic_float::AtomicF32;
use native_dialog::{FileDialog};
use nih_plug::prelude::{util, Editor};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::path::Path;
use std::sync::atomic::{Ordering, AtomicI64};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::params::{TestParams};
use crate::state::TextState;


#[derive(Lens)]
struct Data {
    params: Arc<TestParams>,
    peak_meter: Arc<AtomicF32>,
    t: Arc<TextState>,
    t_id: Arc<AtomicI64>,
}

impl Model for Data {}

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (200, 200))
}

pub(crate) fn create(
    params: Arc<TestParams>,
    t: Arc<TextState>,
    t_id: Arc<AtomicI64>,
    peak_meter: Arc<AtomicF32>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        Data {
            params: params.clone(),
            peak_meter: peak_meter.clone(),
            t: t.clone(),
            t_id: t_id.clone(),
        }
        .build(cx);

        ResizeHandle::new(cx);

        VStack::new(cx, |cx| {
            {
                let t = t.clone();
                let t_id = t_id.clone();
                Button::new(cx,  move |_| {
                    let t = t.clone();
                    let t_id = t_id.clone();
                    thread::spawn(move || {
                        let mut loc = t.get_v();
                        let last_loc_exists = Path::new(&loc).exists();
                        if !last_loc_exists {
                            loc = "~/Desktop".to_string();
                        }
                        

                        let path = FileDialog::new()
                            .set_location(&loc)
                            .add_filter(".WAV Image :3c", &["wav"])
                            .show_open_single_file()
                            .ok().unwrap_or(None);
    
                        if let Some(path) = path {
                            let path = path.as_path().to_str().unwrap_or_default().to_string();
                            t.set_v(path.to_string());
                            t_id.store(rand::random(), Ordering::Relaxed);
                        }
                    });
                }, |cx| {
                    Label::new(cx, Data::t.map(|p| {
                        let loc = p.get_v();
                        let loc = Path::new(&loc);
                        let default_loc = "<default>".to_string();
                        return if !loc.is_file() {
                            default_loc
                        } else if let Some(n) = loc.file_name() { if let Some(n) = n.to_str() {
                            n.to_string()
                        } else { default_loc } }
                        else { default_loc }
                    }))
                });
            }

            Label::new(cx, "Gain GUI")
                .font_family(vec![FamilyOwned::Name(String::from(
                    assets::NOTO_SANS_THIN,
                ))])
                .font_size(30.0)
                .height(Pixels(50.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));

            Label::new(cx, "Gain");
            ParamSlider::new(cx, Data::params, |params| &params.gain);

            PeakMeter::new(
                cx,
                Data::peak_meter
                    .map(|peak_meter| util::gain_to_db(peak_meter.load(Ordering::Relaxed))),
                Some(Duration::from_millis(600)),
            )
            // This is how adding padding works in vizia
            .top(Pixels(10.0));
        })
        .row_between(Pixels(0.0))
        .child_left(Stretch(1.0))
        .child_right(Stretch(1.0));
    })
}
