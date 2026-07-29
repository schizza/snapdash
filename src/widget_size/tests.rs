use crate::ha::{Axis, AxisKind, Control};
use crate::helpers::humanize_magnitude;
use crate::widget_size::WidgetSize;

fn axis(kind: AxisKind, max: f32) -> Axis {
    Axis {
        kind,
        min: 0.0,
        max,
        step: 1.0,
        current: None,
    }
}

fn slider() -> Control {
    Control::Value(axis(AxisKind::Brightness, 255.0))
}

fn colour() -> Control {
    Control::Color {
        hue: axis(AxisKind::Hue, 359.0),
        saturation: axis(AxisKind::Saturation, 100.0),
    }
}

/// The three the design calls for, stated as literals rather than
/// recomputed from the presets: they are the numbers the field was drawn
/// to, and deriving them here would make this test agree with whatever
/// the arithmetic happens to say.
#[test]
fn the_colour_field_is_a_slider_track_wide_and_half_as_tall() {
    assert_eq!(
        WidgetSize::Small.colour_field_size(),
        iced::Size::new(132.0, 66.0)
    );
    assert_eq!(
        WidgetSize::Normal.colour_field_size(),
        iced::Size::new(172.0, 86.0)
    );
    assert_eq!(
        WidgetSize::Large.colour_field_size(),
        iced::Size::new(212.0, 106.0)
    );
}

/// A colour surface is a field, not a slider, so the window grows by
/// more for one than for the other. That is the whole reason the
/// controls area is a sum over per-control heights rather than a row
/// height times a count.
#[test]
fn a_colour_surface_costs_more_height_than_a_slider() {
    for &size in WidgetSize::ALL {
        let field = size.colour_field_size().height;

        assert!(
            size.control_height(&colour()) > size.control_height(&slider()),
            "{size}"
        );
        // The difference is the field standing where the rail would, and
        // nothing else: the label line and the separating gap are shared.
        assert_eq!(
            size.control_height(&colour()) - size.control_height(&slider()),
            field - 16.0,
            "{size}"
        );
    }
}

/// The mixture a colour bulb actually offers: a brightness slider, a
/// white slider and the colour surface, each with its own gap.
#[test]
fn a_colour_bulbs_controls_sum_their_own_heights() {
    for &size in WidgetSize::ALL {
        assert_eq!(
            size.controls_height(&[slider(), slider(), colour()]),
            size.control_height(&slider()) * 2.0 + size.control_height(&colour()),
            "{size}"
        );
    }
}

/// An entity with no control has nothing to reveal, so there is nothing
/// to grow into and the widget stays at its preset.
#[test]
fn an_entity_with_no_controls_grows_by_nothing() {
    for &size in WidgetSize::ALL {
        assert_eq!(size.controls_height(&[]), 0.0, "{size}");
    }
}

/// `entity_window` puts a separating gap in front of every control,
/// so the height each one costs is the gap plus its body. A widget that
/// counted one gap for the whole block came up short as soon as it had a
/// second control, and the last slider paid for it.
#[test]
fn every_control_costs_its_own_gap() {
    for &size in WidgetSize::ALL {
        let one = size.controls_height(&[slider()]);
        assert_eq!(
            one,
            size.value_detail_gap() + size.control_row_height(),
            "{size}"
        );
        assert_eq!(
            size.controls_height(&[slider(), slider()]),
            one * 2.0,
            "{size}"
        );
        assert_eq!(
            size.controls_height(&[slider(), slider(), slider()]),
            one * 3.0,
            "{size}"
        );
    }
}

#[test]
fn compresses_large_watts() {
    assert_eq!(humanize_magnitude("1234567 W"), "1.23 MW");
    assert_eq!(humanize_magnitude("5000 W"), "5 kW");
}

#[test]
fn skips_in_range() {
    assert_eq!(humanize_magnitude("500 W"), "500 W");
    assert_eq!(humanize_magnitude("23 V"), "23 V");
}

#[test]
fn skips_unsupported_units() {
    assert_eq!(humanize_magnitude("23.5 °C"), "23.5 °C");
    assert_eq!(humanize_magnitude("45.2 %"), "45.2 %");
}

#[test]
fn skips_already_prefixed() {
    assert_eq!(humanize_magnitude("1.5 kW"), "1.5 kW");
}

#[test]
fn handles_non_numeric() {
    assert_eq!(humanize_magnitude("On"), "On");
    assert_eq!(humanize_magnitude(""), "");
}

#[test]
fn compresses_small() {
    assert_eq!(humanize_magnitude("0.0005 A"), "500 µA");
}
#[test]
fn does_not_scale_moderately_small_values() {
    assert_eq!(humanize_magnitude("0.1 A"), "0.1 A");
    assert_eq!(humanize_magnitude("0.5 A"), "0.5 A");
    assert_eq!(humanize_magnitude("0.05 A"), "0.05 A");
}

#[test]
fn scales_truly_small_values() {
    assert_eq!(humanize_magnitude("0.005 A"), "5 mA");
    assert_eq!(humanize_magnitude("0.0001 A"), "100 µA");
    assert_eq!(humanize_magnitude("0.0051 A"), "5.1 mA");
}

#[test]
fn trims_trailing_zeros() {
    assert_eq!(humanize_magnitude("5000 W"), "5 kW");
    assert_eq!(humanize_magnitude("1500 W"), "1.5 kW");
    assert_eq!(humanize_magnitude("1230000 W"), "1.23 MW");
}

#[test]
fn scales_into_nano_and_pico() {
    assert_eq!(humanize_magnitude("0.000000001 A"), "1 nA"); // 1e-9
    assert_eq!(humanize_magnitude("0.0000000005 A"), "500 pA"); // 5e-10
    assert_eq!(humanize_magnitude("0.000000000001 A"), "1 pA"); // 1e-12
}

#[test]
fn boundary_between_milli_and_micro() {
    assert_eq!(humanize_magnitude("0.001 A"), "1 mA");
    assert_eq!(humanize_magnitude("0.0001 A"), "100 µA"); // dropped from milli to micro
}
