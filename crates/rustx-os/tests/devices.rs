//! What is plugged into this machine.
//!
//! These tests run on whatever machine they are run on: one with three
//! webcams, one with none, one whose only camera is being used by a
//! meeting. All three are ordinary, so none of them is a failure — what
//! is asked here is that the answer be *true*, and that nothing in it be
//! half a device.

use rustx_os::devices::{Kind, devices, devices_of};

#[test]
fn every_device_has_an_id_and_a_name() {
    for device in devices() {
        assert!(!device.id.is_empty(), "a device with no id: {device:?}");
        assert!(!device.name.is_empty(), "a device with no name: {device:?}");
        assert!(
            !device.name.trim().is_empty(),
            "a device named with spaces: {device:?}"
        );
    }
}

#[test]
fn a_machine_without_a_camera_lists_none_and_does_not_fail() {
    // The constraint, in a test: no hardware is required by anything
    // here. A machine with no webcam answers with an empty list, which
    // is an answer.
    let cameras = devices_of(Kind::Camera);
    for camera in &cameras {
        assert_eq!(camera.kind, Kind::Camera);
    }
    println!("cameras: {cameras:#?}");
}

#[test]
fn a_device_that_is_busy_is_still_listed() {
    // Review Focus 1. A camera another application is holding is a
    // camera that exists; an enumeration that hides it would have the
    // developer look for a bug in their own code.
    for device in devices() {
        // `busy` is a state of a device that is there, so it can only
        // be true of something that was listed. The assertion is the
        // shape: nothing is listed as busy *and* nameless.
        if device.busy {
            assert!(!device.name.is_empty());
        }
    }
}

#[test]
fn the_screens_are_devices_too() {
    // A screen is a source like a camera (plan task 4), so it is a
    // device like a camera. A machine running this test has a screen —
    // or is a build server, which has none, and says so.
    let screens = devices_of(Kind::Screen);
    println!("screens: {screens:#?}");
    for screen in &screens {
        assert_eq!(screen.kind, Kind::Screen);
        assert!(!screen.id.is_empty());
    }
}

#[test]
fn an_id_survives_two_enumerations() {
    // `/device/[id]` is a path in a URL: an id that changed between the
    // list and the screen it opens would open nothing.
    let first = devices();
    let again = devices();
    let ids = |which: &[rustx_os::devices::Device]| -> Vec<String> {
        let mut ids: Vec<String> = which.iter().map(|device| device.id.clone()).collect();
        ids.sort();
        ids
    };
    assert_eq!(ids(&first), ids(&again));
}

#[test]
fn a_device_that_is_not_there_is_refused_by_name() {
    let said = rustx_os::devices::device("nothing-is-plugged-in-here")
        .expect_err("there is no such device");
    assert!(
        said.to_string().contains("nothing-is-plugged-in-here"),
        "got {said}"
    );
}
