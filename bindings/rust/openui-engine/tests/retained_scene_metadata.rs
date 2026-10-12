//! Owned metadata guards. These exercise the native Engine and canonical CPU
//! path; source admission alone is not runtime or Chromium qualification.

use openui_compositor::{
    SceneBoxRole, SceneLayerSnapshot, SceneNodeIdentity, SceneRecordingRole, SceneSnapshot,
    SoftwareCompositor,
};
use openui_dom::{Document, ElementTag};
use openui_engine::{Engine, FontFaceDescriptor, NodeHandle, ViewportMetrics};
use openui_style::{
    Color, Display, FontFamily, FontFamilyList, LengthValue, Overflow, RendererStyleValue,
    StyleProperty, StyleValue,
};
use std::sync::Arc;

fn engine() -> Engine {
    Engine::new(ViewportMetrics::from_logical_size(160.0, 96.0, 1.25).unwrap()).unwrap()
}

fn box_node(engine: &mut Engine, parent: NodeHandle, tag: ElementTag) -> NodeHandle {
    let node = engine.create_native_element(tag).unwrap();
    engine.append_child(parent, node).unwrap();
    if tag == ElementTag::Div {
        engine
            .set_property(node, StyleProperty::Display, Display::Block.into())
            .unwrap();
    }
    for (property, value) in [(StyleProperty::Width, 60.0), (StyleProperty::Height, 40.0)] {
        engine
            .set_property(node, property, LengthValue::px(value).into())
            .unwrap();
    }
    node
}

fn identity(node: NodeHandle) -> SceneNodeIdentity {
    SceneNodeIdentity {
        document: node.document_id(),
        index: node.index(),
        generation: node.generation(),
    }
}

fn layer(scene: &SceneSnapshot, node: NodeHandle) -> &SceneLayerSnapshot {
    scene
        .retained_scene()
        .unwrap()
        .layers()
        .iter()
        .find(|layer| layer.owner == identity(node))
        .unwrap()
}

#[test]
fn identities_survive_geometry_effect_scroll_and_hidden_projection_changes() {
    let mut engine = engine();
    let root = engine.root();
    let node = box_node(&mut engine, root, ElementTag::Div);
    engine
        .set_property(node, StyleProperty::Overflow, Overflow::Hidden.into())
        .unwrap();
    let child = box_node(&mut engine, node, ElementTag::Div);
    engine
        .set_property(child, StyleProperty::Width, LengthValue::px(200.0).into())
        .unwrap();
    let first = engine.scene().unwrap();
    let original = layer(&first, node).clone();
    assert!(original.scroll.is_some());
    let stats = engine.stats();
    let unchanged = engine.scene().unwrap();
    assert_eq!(engine.stats(), stats);
    assert!(Arc::ptr_eq(
        first.retained_scene().unwrap(),
        unchanged.retained_scene().unwrap()
    ));

    engine
        .set_property(node, StyleProperty::Width, LengthValue::px(70.0).into())
        .unwrap();
    engine
        .set_property(node, StyleProperty::Opacity, StyleValue::Number(0.25))
        .unwrap();
    engine.scroll_to(node, 10.0, 0.0).unwrap();
    let changed = engine.scene().unwrap();
    let updated = layer(&changed, node);
    assert_eq!(
        (original.id, original.effect, original.clip, original.scroll),
        (updated.id, updated.effect, updated.clip, updated.scroll)
    );
    assert_ne!(
        original.projections[0].border_box,
        updated.projections[0].border_box
    );
    let effect = |scene: &SceneSnapshot| {
        scene
            .retained_scene()
            .unwrap()
            .effects()
            .iter()
            .find(|effect| effect.id == original.effect)
            .unwrap()
            .opacity
    };
    assert_eq!(effect(&first), 1.0);
    assert_eq!(effect(&changed), 0.25);
    let old_scroll = first
        .retained_scene()
        .unwrap()
        .scrolls()
        .iter()
        .find(|scroll| Some(scroll.id) == original.scroll)
        .unwrap();
    let new_scroll = changed
        .retained_scene()
        .unwrap()
        .scrolls()
        .iter()
        .find(|scroll| Some(scroll.id) == original.scroll)
        .unwrap();
    assert_eq!(old_scroll.projections[0].offset, (0.0, 0.0));
    assert_eq!(new_scroll.projections[0].offset, (10.0, 0.0));

    engine
        .set_property(node, StyleProperty::Overflow, Overflow::Visible.into())
        .unwrap();
    let unclipped = engine.scene().unwrap();
    assert_eq!(layer(&unclipped, node).clip, original.clip);
    assert!(layer(&unclipped, node).scroll.is_none());
    let clip = unclipped
        .retained_scene()
        .unwrap()
        .clips()
        .iter()
        .find(|clip| clip.id == original.clip)
        .unwrap();
    assert!(!clip.projections[0].overflow_clips_x && !clip.projections[0].overflow_clips_y);
    let old_clip = first
        .retained_scene()
        .unwrap()
        .clips()
        .iter()
        .find(|clip| clip.id == original.clip)
        .unwrap();
    assert!(old_clip.projections[0].overflow_clips_x && old_clip.projections[0].overflow_clips_y);
    engine
        .set_property(node, StyleProperty::Overflow, Overflow::Hidden.into())
        .unwrap();
    assert_eq!(
        layer(&engine.scene().unwrap(), node).scroll,
        original.scroll
    );

    engine
        .set_property(node, StyleProperty::Display, Display::None.into())
        .unwrap();
    let hidden = engine.scene().unwrap();
    assert!(hidden
        .retained_scene()
        .unwrap()
        .layers()
        .iter()
        .all(|layer| layer.owner != identity(node)));
    engine
        .set_property(node, StyleProperty::Display, Display::Block.into())
        .unwrap();
    assert_eq!(layer(&engine.scene().unwrap(), node).id, original.id);
    engine.detach(node).unwrap();
    engine.scene().unwrap();
    engine.append_child(root, node).unwrap();
    assert_eq!(layer(&engine.scene().unwrap(), node).id, original.id);
    let new_parent = box_node(&mut engine, root, ElementTag::Div);
    engine.append_or_move_child(new_parent, node).unwrap();
    let reparented = engine.scene().unwrap();
    assert_eq!(layer(&reparented, node).id, original.id);
    assert_eq!(
        layer(&reparented, node).projections[0].layout_parent,
        Some(layer(&reparented, new_parent).id)
    );
    assert_ne!(
        original.projections[0].layout_parent,
        layer(&reparented, node).projections[0].layout_parent
    );
}

#[test]
fn reused_handle_slots_and_other_documents_never_reuse_scene_identities() {
    let mut engine = engine();
    let root = engine.root();
    let old = box_node(&mut engine, root, ElementTag::Div);
    let saved = engine.scene().unwrap();
    let previous = layer(&saved, old).clone();
    engine.remove(old).unwrap();
    let replacement = box_node(&mut engine, root, ElementTag::Div);
    assert_eq!(old.index(), replacement.index());
    assert_ne!(old.generation(), replacement.generation());
    let replacement_scene = engine.scene().unwrap();
    let next = layer(&replacement_scene, replacement);
    assert_ne!(previous.id, next.id);
    assert_ne!(previous.effect, next.effect);
    assert_ne!(previous.clip, next.clip);
    assert_eq!(layer(&saved, old).owner, identity(old));
    assert!(replacement_scene
        .retained_scene()
        .unwrap()
        .layers()
        .iter()
        .all(|layer| layer.owner != identity(old)));

    let mut other = Engine::new(engine.viewport()).unwrap();
    let other_root = other.root();
    let other_node = box_node(&mut other, other_root, ElementTag::Div);
    let other_scene = other.scene().unwrap();
    assert_ne!(next.id, layer(&other_scene, other_node).id);
    let canonical = |scene: &SceneSnapshot| {
        scene
            .retained_scene()
            .unwrap()
            .recordings()
            .iter()
            .find(|recording| recording.role() == SceneRecordingRole::Canonical)
            .unwrap()
            .id()
    };
    assert_ne!(canonical(&replacement_scene), canonical(&other_scene));
}

#[test]
fn borrowed_table_helpers_cannot_duplicate_owned_effects_or_scroll_owners() {
    let mut engine = engine();
    let root = engine.root();
    let table = box_node(&mut engine, root, ElementTag::Table);
    let caption = box_node(&mut engine, table, ElementTag::TableCaption);
    let row = engine.create_native_element(ElementTag::TableRow).unwrap();
    engine.append_child(table, row).unwrap();
    let cell = box_node(&mut engine, row, ElementTag::TableCell);
    engine
        .set_property(table, StyleProperty::Opacity, StyleValue::Number(0.5))
        .unwrap();
    let scene = engine.scene().unwrap();
    let retained = scene.retained_scene().unwrap();
    assert!(!retained.authored_layers_are_partitioned());
    assert!(retained.unidentified_owned_box_paths().is_empty());
    for node in [table, caption, row, cell] {
        assert_eq!(
            retained
                .layers()
                .iter()
                .filter(|layer| layer.owner == identity(node))
                .count(),
            1
        );
    }
    for projection in &*layer(&scene, table).projections {
        let mut fragment = scene.fragments().as_ref();
        for index in projection.fragment_path.iter() {
            fragment = &fragment.children[*index];
        }
        assert!(fragment.owns_element_box());
        assert_eq!(projection.border_box, fragment.border_box_rect());
    }
    assert_eq!(
        retained
            .effects()
            .iter()
            .filter(|effect| effect.owner == identity(table))
            .count(),
        1
    );
    // No whole-document recording is attached to an authored node plan.
    assert!(retained
        .recordings()
        .iter()
        .all(|recording| recording.id() != layer(&scene, table).id));
}

#[test]
fn recording_and_scene_resources_survive_engine_and_scene_destruction() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<SceneSnapshot>();
    assert_send_sync::<openui_compositor::SceneLayerRecording>();
    let mut engine = engine();
    let font = engine
        .register_font_face(
            Arc::from(include_bytes!("../../openui-text/fonts/Ahem.ttf").as_slice()),
            FontFaceDescriptor::new("Retained scene Ahem"),
        )
        .unwrap();
    let root = engine.root();
    engine
        .set_property(
            root,
            StyleProperty::FontFamily,
            FontFamilyList {
                families: vec![FontFamily::Named("Retained scene Ahem".into())],
            }
            .into(),
        )
        .unwrap();
    let text = engine.create_text("Owned").unwrap();
    engine.append_child(root, text).unwrap();
    let image = box_node(&mut engine, root, ElementTag::Image);
    let resource = engine.register_image_resource(
        "memory:retained-layer", "image/svg+xml",
        "809bab3d90547e75eda8d473aa72a4f4f20b9e23ea7885f2e900eb0cf61aa6de",
        br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" fill="#008080"/></svg>"##.to_vec(),
    );
    engine
        .set_image_resource(image, resource, Some((1.0, 1.0)))
        .unwrap();
    let scene = engine.scene().unwrap();
    let mut compositor = SoftwareCompositor::default();
    let expected = compositor.render(&scene).unwrap();
    assert!(
        expected
            .pixels
            .chunks_exact(4)
            .any(|pixel| pixel == [0, 128, 128, 255]),
        "the owned image resource must contribute real pixels"
    );
    assert_eq!(
        compositor.render(&engine.scene().unwrap()).unwrap(),
        expected
    );
    assert_eq!(compositor.stats().reused, 1);
    let recording = scene
        .retained_scene()
        .unwrap()
        .recordings()
        .iter()
        .find(|recording| recording.role() == SceneRecordingRole::Canonical)
        .unwrap()
        .clone();
    let serialized = recording.picture().serialize().as_bytes().to_vec();
    assert_eq!(recording.retained_font_face_count(), 1);
    engine.unregister_font_face(font).unwrap();
    engine.remove(image).unwrap();
    engine.set_text(text, "New live value").unwrap();
    drop(engine);
    drop(compositor);
    let (frame, detached_recording) = std::thread::spawn(move || {
        let frame = SoftwareCompositor::default().render(&scene).unwrap();
        drop(scene);
        assert_eq!(recording.retained_font_face_count(), 1);
        (frame, recording.picture().serialize().as_bytes().to_vec())
    })
    .join()
    .unwrap();
    assert_eq!(frame, expected);
    assert_eq!(detached_recording, serialized);
}

#[test]
fn existing_scene_constructor_retains_its_metadata_free_cpu_contract() {
    let viewport = ViewportMetrics::from_logical_size(32.0, 24.0, 1.25).unwrap();
    let document = Document::new();
    let (fragment, recording) = openui_paint::record_document(&document, viewport).unwrap();
    let scene = SceneSnapshot::new(
        openui_compositor::SceneGeneration(1),
        recording,
        Arc::new(fragment),
        Vec::new(),
    );
    assert!(scene.retained_scene().is_none());
    let mut compositor = SoftwareCompositor::default();
    let first = compositor.render(&scene).unwrap();
    assert_eq!(compositor.render(&scene).unwrap(), first);
    assert_eq!(compositor.stats().rasterized, 1);
    assert_eq!(compositor.stats().reused, 1);
    drop(document);
    assert_eq!(SoftwareCompositor::default().render(&scene).unwrap(), first);
}

#[test]
fn viewport_recordings_have_distinct_stable_roles_and_owned_scroll_geometry() {
    let mut engine = engine();
    let root = engine.root();
    engine
        .set_property(root, StyleProperty::Overflow, Overflow::Scroll.into())
        .unwrap();
    let child = box_node(&mut engine, root, ElementTag::Div);
    engine
        .set_property(child, StyleProperty::Width, LengthValue::px(240.0).into())
        .unwrap();
    let first = engine.scene().unwrap();
    let retained = first.retained_scene().unwrap();
    let roles: Vec<_> = retained
        .recordings()
        .iter()
        .map(|recording| recording.role())
        .collect();
    assert_eq!(
        roles,
        [
            SceneRecordingRole::Canonical,
            SceneRecordingRole::DocumentContent,
            SceneRecordingRole::ViewportScrollingContent,
            SceneRecordingRole::ViewportOverlay
        ]
    );
    for (index, recording) in retained.recordings().iter().enumerate() {
        assert!(retained.recordings()[..index]
            .iter()
            .all(|other| other.id() != recording.id()));
        assert!(recording.picture().approximate_op_count() > 0);
    }
    let root_scroll = layer(&first, root).scroll.unwrap();
    let scroll = retained
        .scrolls()
        .iter()
        .find(|scroll| scroll.id == root_scroll)
        .unwrap();
    assert!(scroll.projections[0].viewport);
    assert!(!scroll.projections[0].area.integer_offsets);
    engine.scroll_to(root, 10.0, 0.0).unwrap();
    let second = engine.scene().unwrap();
    assert_eq!(layer(&second, root).scroll, Some(root_scroll));
    for (old, new) in retained
        .recordings()
        .iter()
        .zip(second.retained_scene().unwrap().recordings())
    {
        assert_eq!((old.id(), old.role()), (new.id(), new.role()));
    }
    assert_eq!(scroll.projections[0].offset, (0.0, 0.0));
    let second_scroll = second
        .retained_scene()
        .unwrap()
        .scrolls()
        .iter()
        .find(|scroll| scroll.id == root_scroll)
        .unwrap();
    assert_eq!(second_scroll.projections[0].offset, (10.0, 0.0));
}

#[test]
fn fieldset_content_owns_separate_scroll_clip_scope_without_legend_or_effect_duplication() {
    let mut engine = engine();
    let root = engine.root();
    let fieldset = box_node(&mut engine, root, ElementTag::Fieldset);
    engine
        .set_property(
            fieldset,
            StyleProperty::MinWidth,
            LengthValue::px(0.0).into(),
        )
        .unwrap();
    engine
        .set_property(fieldset, StyleProperty::Opacity, StyleValue::Number(0.5))
        .unwrap();
    engine
        .set_property(fieldset, StyleProperty::Overflow, Overflow::Hidden.into())
        .unwrap();
    let legend = box_node(&mut engine, fieldset, ElementTag::Legend);
    engine
        .set_property(legend, StyleProperty::Width, LengthValue::px(30.0).into())
        .unwrap();
    engine
        .set_property(legend, StyleProperty::Height, LengthValue::px(10.0).into())
        .unwrap();
    let content = box_node(&mut engine, fieldset, ElementTag::Div);
    engine
        .set_property(
            content,
            StyleProperty::Height,
            LengthValue::px(200.0).into(),
        )
        .unwrap();
    let first = engine.scene().unwrap();
    let retained = first.retained_scene().unwrap();
    assert!(retained.unidentified_owned_box_paths().is_empty());
    let wrapper = layer(&first, fieldset);
    let content_clip_id = wrapper.content_clip.unwrap();
    let content_scroll_id = wrapper.scroll.unwrap();
    assert_ne!(wrapper.clip, content_clip_id);
    let clips = retained
        .clips()
        .iter()
        .find(|clip| clip.id == content_clip_id)
        .unwrap();
    assert_eq!(clips.role, SceneBoxRole::FieldsetContent);
    assert_eq!(clips.owner, identity(fieldset));
    let scroll = retained
        .scrolls()
        .iter()
        .find(|scroll| scroll.id == content_scroll_id)
        .unwrap();
    assert_eq!(scroll.role, SceneBoxRole::FieldsetContent);
    assert_eq!(scroll.owner, identity(fieldset));
    assert!(clips.projections[0].overflow_clips_y);
    assert!(!clips.projections[0].paint_containment);
    assert!(clips.projections[0].clip_path_insets.is_none());
    let principal_clip = retained
        .clips()
        .iter()
        .find(|clip| clip.id == wrapper.clip)
        .unwrap();
    assert!(!principal_clip.projections[0].overflow_clips_x);
    assert!(!principal_clip.projections[0].overflow_clips_y);
    let helper_path = &clips.projections[0].fragment_path;
    assert!(layer(&first, content).projections[0]
        .fragment_path
        .starts_with(helper_path));
    assert!(!layer(&first, legend).projections[0]
        .fragment_path
        .starts_with(helper_path));
    let mut helper = first.fragments().as_ref();
    for index in helper_path.iter() {
        helper = &helper.children[*index];
    }
    assert!(!helper.owns_element_box());
    assert!(helper.owns_scroll_box());
    assert_eq!(
        clips.projections[0].client_rect,
        helper.scroll_area.unwrap().client_rect
    );
    assert_eq!(clips.projections[0].offset, helper.offset);
    assert_eq!(scroll.projections[0].box_offset, helper.offset);
    assert_eq!(
        clips.projections[0].border_radius_percent,
        helper
            .layout_style(helper.inherited_style.as_ref().unwrap())
            .border_radius_percent
    );
    assert_eq!(
        retained
            .layers()
            .iter()
            .filter(|layer| layer.owner == identity(fieldset))
            .count(),
        1
    );
    assert_eq!(
        retained
            .effects()
            .iter()
            .filter(|effect| effect.owner == identity(fieldset))
            .count(),
        1
    );
    engine.scroll_to(fieldset, 0.0, 10.0).unwrap();
    engine
        .set_property(fieldset, StyleProperty::Width, LengthValue::px(70.0).into())
        .unwrap();
    engine
        .set_property(legend, StyleProperty::Display, Display::None.into())
        .unwrap();
    let changed = engine.scene().unwrap();
    let next = layer(&changed, fieldset);
    assert_eq!(
        (
            next.id,
            next.effect,
            next.clip,
            next.content_clip,
            next.scroll
        ),
        (
            wrapper.id,
            wrapper.effect,
            wrapper.clip,
            wrapper.content_clip,
            wrapper.scroll
        )
    );
    let next_scroll = changed
        .retained_scene()
        .unwrap()
        .scrolls()
        .iter()
        .find(|scroll| scroll.id == content_scroll_id)
        .unwrap();
    assert_eq!(next_scroll.projections[0].offset, (0.0, 10.0));
    assert_eq!(scroll.projections[0].offset, (0.0, 0.0));
    assert_ne!(
        clips.projections[0].fragment_path,
        changed
            .retained_scene()
            .unwrap()
            .clips()
            .iter()
            .find(|clip| clip.id == content_clip_id)
            .unwrap()
            .projections[0]
            .fragment_path
    );
    engine
        .set_property(fieldset, StyleProperty::Display, Display::None.into())
        .unwrap();
    engine.scene().unwrap();
    engine
        .set_property(fieldset, StyleProperty::Display, Display::Block.into())
        .unwrap();
    let visible = engine.scene().unwrap();
    assert_eq!(
        layer(&visible, fieldset).content_clip,
        Some(content_clip_id)
    );
    assert_eq!(layer(&visible, fieldset).scroll, Some(content_scroll_id));
}

#[test]
fn radius_units_remain_owned_and_distinct_from_identical_numeric_pixel_radii() {
    let mut engine = engine();
    let root = engine.root();
    let node = box_node(&mut engine, root, ElementTag::Div);
    engine
        .set_property(node, StyleProperty::BackgroundColor, Color::RED.into())
        .unwrap();
    engine
        .set_property(node, StyleProperty::Overflow, Overflow::Hidden.into())
        .unwrap();
    engine
        .set_property(
            node,
            StyleProperty::BorderTopLeftRadius,
            StyleValue::Renderer(RendererStyleValue::BorderTopLeftRadius((20.0, 20.0))),
        )
        .unwrap();
    let pixels = engine.scene().unwrap();
    let id = layer(&pixels, node).clip;
    engine
        .set_property(
            node,
            StyleProperty::BorderRadiusPercent,
            StyleValue::Renderer(RendererStyleValue::BorderRadiusPercent([(true, true); 4])),
        )
        .unwrap();
    let percent = engine.scene().unwrap();
    assert_eq!(layer(&percent, node).clip, id);
    let old = &pixels
        .retained_scene()
        .unwrap()
        .clips()
        .iter()
        .find(|clip| clip.id == id)
        .unwrap()
        .projections[0];
    let new = &percent
        .retained_scene()
        .unwrap()
        .clips()
        .iter()
        .find(|clip| clip.id == id)
        .unwrap()
        .projections[0];
    assert_eq!(old.border_radii, new.border_radii);
    assert_eq!(old.border_radius_percent, [(false, false); 4]);
    assert_eq!(new.border_radius_percent, [(true, true); 4]);
    let expected_pixels = SoftwareCompositor::default().render(&pixels).unwrap();
    let expected_percent = SoftwareCompositor::default().render(&percent).unwrap();
    assert_ne!(expected_pixels.pixels, expected_percent.pixels);
    drop(engine);
    assert_eq!(
        SoftwareCompositor::default().render(&pixels).unwrap(),
        expected_pixels
    );
    assert_eq!(
        SoftwareCompositor::default().render(&percent).unwrap(),
        expected_percent
    );
}
