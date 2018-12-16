use crate::objects::{Ctx, DEBUG_EXTRA, ID};
use crate::plugins::{Plugin, PluginCtx};
use ezgui::{Color, Key};

pub struct OsmClassifier {}

impl OsmClassifier {
    pub fn new(ctx: &mut PluginCtx) -> Option<OsmClassifier> {
        if ctx
            .input
            .unimportant_key_pressed(Key::Num6, DEBUG_EXTRA, "show OSM colors")
        {
            return Some(OsmClassifier {});
        }
        None
    }
}

impl Plugin for OsmClassifier {
    fn blocking_event(&mut self, ctx: &mut PluginCtx) -> bool {
        if ctx.input.key_pressed(Key::Num6, "stop showing OSM colors") {
            return false;
        }
        true
    }

    fn color_for(&self, obj: ID, ctx: &Ctx) -> Option<Color> {
        match obj {
            ID::Lane(l) => {
                if ctx.map.get_l(l).is_driving() {
                    match ctx
                        .map
                        .get_parent(l)
                        .osm_tags
                        .get("highway")
                        .map(|s| s.as_str())
                    {
                        // From https://wiki.openstreetmap.org/wiki/Map_Features#Highway
                        Some("motorway") | Some("motorway_link") => {
                            Some(ctx.cs.get_def("OSM motorway", Color::rgb(231, 141, 159)))
                        }
                        Some("trunk") | Some("trunk_link") => {
                            Some(ctx.cs.get_def("OSM trunk", Color::rgb(249, 175, 152)))
                        }
                        Some("primary") | Some("primary_link") => {
                            Some(ctx.cs.get_def("OSM primary", Color::rgb(252, 213, 160)))
                        }
                        Some("secondary") | Some("secondary_link") => {
                            Some(ctx.cs.get_def("OSM secondary", Color::rgb(252, 213, 160)))
                        }
                        Some("residential") => {
                            Some(ctx.cs.get_def("OSM residential", Color::rgb(254, 254, 254)))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            ID::Building(b) => {
                if ctx.map.get_b(b).osm_tags.contains_key("addr:housenumber") {
                    Some(ctx.cs.get_def("OSM house", Color::GREEN))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
