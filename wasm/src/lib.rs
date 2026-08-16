use serde::Serialize;
use std::f32::consts::PI;
use wasm_bindgen::prelude::*;

const COPPER_THICKNESS: f32 = 0.035;
const SILK_THICKNESS: f32 = 0.018;
const EPSILON: f32 = 0.01;

#[derive(Debug, Clone)]
enum SExpr {
    Atom(String),
    List(Vec<SExpr>),
}

#[derive(Debug, Clone, Copy, Default)]
struct Point2 {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct Point3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeometryBundle {
    meshes: Vec<MeshData>,
    stats: BoardStats,
    bounds: BoardBounds,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MeshData {
    name: String,
    material: String,
    layer: String,
    positions: Vec<f32>,
    normals: Vec<f32>,
    indices: Vec<u32>,
}

#[derive(Debug, Serialize)]
struct BoardStats {
    components: usize,
    pads: usize,
    tracks: usize,
    vias: usize,
    layers: usize,
}

#[derive(Debug, Serialize)]
struct BoardBounds {
    width: f32,
    height: f32,
    thickness: f32,
}

#[derive(Default)]
struct MeshBuilder {
    positions: Vec<f32>,
    normals: Vec<f32>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    fn vertex(&mut self, p: Point3, n: Point3) -> u32 {
        let index = (self.positions.len() / 3) as u32;
        self.positions.extend_from_slice(&[p.x, p.y, p.z]);
        self.normals.extend_from_slice(&[n.x, n.y, n.z]);
        index
    }

    fn triangle(&mut self, a: Point3, mut b: Point3, mut c: Point3, normal: Point3) {
        let cross = cross3(sub3(b, a), sub3(c, a));
        if dot3(cross, normal) < 0.0 {
            std::mem::swap(&mut b, &mut c);
        }
        let ia = self.vertex(a, normal);
        let ib = self.vertex(b, normal);
        let ic = self.vertex(c, normal);
        self.indices.extend_from_slice(&[ia, ib, ic]);
    }

    fn quad(&mut self, a: Point3, mut b: Point3, c: Point3, mut d: Point3, normal: Point3) {
        let cross = cross3(sub3(b, a), sub3(c, a));
        if dot3(cross, normal) < 0.0 {
            std::mem::swap(&mut b, &mut d);
        }
        let ia = self.vertex(a, normal);
        let ib = self.vertex(b, normal);
        let ic = self.vertex(c, normal);
        let id = self.vertex(d, normal);
        self.indices.extend_from_slice(&[ia, ib, ic, ia, ic, id]);
    }

    fn add_segment_prism(
        &mut self,
        start: Point2,
        end: Point2,
        center_y: f32,
        width: f32,
        height: f32,
    ) {
        let dx = end.x - start.x;
        let dz = -(end.y - start.y);
        let length = (dx * dx + dz * dz).sqrt();
        if length <= f32::EPSILON {
            return;
        }
        let ux = dx / length;
        let uz = dz / length;
        let px = -uz * width * 0.5;
        let pz = ux * width * 0.5;
        let y0 = center_y - height * 0.5;
        let y1 = center_y + height * 0.5;
        let s = Point2 {
            x: start.x,
            y: -start.y,
        };
        let e = Point2 {
            x: end.x,
            y: -end.y,
        };

        let a0 = Point3 {
            x: s.x + px,
            y: y0,
            z: s.y + pz,
        };
        let b0 = Point3 {
            x: e.x + px,
            y: y0,
            z: e.y + pz,
        };
        let c0 = Point3 {
            x: e.x - px,
            y: y0,
            z: e.y - pz,
        };
        let d0 = Point3 {
            x: s.x - px,
            y: y0,
            z: s.y - pz,
        };
        let a1 = Point3 { y: y1, ..a0 };
        let b1 = Point3 { y: y1, ..b0 };
        let c1 = Point3 { y: y1, ..c0 };
        let d1 = Point3 { y: y1, ..d0 };

        self.quad(
            a1,
            b1,
            c1,
            d1,
            Point3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        );
        self.quad(
            d0,
            c0,
            b0,
            a0,
            Point3 {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        );
        self.quad(
            a0,
            b0,
            b1,
            a1,
            Point3 {
                x: -px * 2.0 / width,
                y: 0.0,
                z: -pz * 2.0 / width,
            },
        );
        self.quad(
            d1,
            c1,
            c0,
            d0,
            Point3 {
                x: px * 2.0 / width,
                y: 0.0,
                z: pz * 2.0 / width,
            },
        );
        self.quad(
            a0,
            a1,
            d1,
            d0,
            Point3 {
                x: -ux,
                y: 0.0,
                z: -uz,
            },
        );
        self.quad(
            b1,
            b0,
            c0,
            c1,
            Point3 {
                x: ux,
                y: 0.0,
                z: uz,
            },
        );
    }

    fn add_oriented_box(
        &mut self,
        center: Point2,
        center_y: f32,
        width: f32,
        depth: f32,
        height: f32,
        rotation_deg: f32,
    ) {
        let angle = rotation_deg.to_radians();
        let half = width * 0.5;
        let dir = Point2 {
            x: angle.cos() * half,
            y: angle.sin() * half,
        };
        self.add_segment_prism(
            Point2 {
                x: center.x - dir.x,
                y: center.y - dir.y,
            },
            Point2 {
                x: center.x + dir.x,
                y: center.y + dir.y,
            },
            center_y,
            depth,
            height,
        );
    }

    fn add_cylinder(
        &mut self,
        center: Point2,
        center_y: f32,
        radius: f32,
        height: f32,
        segments: usize,
    ) {
        if radius <= 0.0 || height <= 0.0 || segments < 3 {
            return;
        }
        let y0 = center_y - height * 0.5;
        let y1 = center_y + height * 0.5;
        let cz = -center.y;
        let top_center = Point3 {
            x: center.x,
            y: y1,
            z: cz,
        };
        let bottom_center = Point3 {
            x: center.x,
            y: y0,
            z: cz,
        };

        for i in 0..segments {
            let a0 = i as f32 * 2.0 * PI / segments as f32;
            let a1 = (i + 1) as f32 * 2.0 * PI / segments as f32;
            let p0 = Point3 {
                x: center.x + radius * a0.cos(),
                y: y0,
                z: cz + radius * a0.sin(),
            };
            let p1 = Point3 {
                x: center.x + radius * a1.cos(),
                y: y0,
                z: cz + radius * a1.sin(),
            };
            let t0 = Point3 { y: y1, ..p0 };
            let t1 = Point3 { y: y1, ..p1 };
            self.triangle(
                top_center,
                t0,
                t1,
                Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            );
            self.triangle(
                bottom_center,
                p1,
                p0,
                Point3 {
                    x: 0.0,
                    y: -1.0,
                    z: 0.0,
                },
            );
            let mid = (a0 + a1) * 0.5;
            self.quad(
                p0,
                p1,
                t1,
                t0,
                Point3 {
                    x: mid.cos(),
                    y: 0.0,
                    z: mid.sin(),
                },
            );
        }
    }

    fn finish(self, name: &str, material: &str, layer: &str) -> MeshData {
        MeshData {
            name: name.to_owned(),
            material: material.to_owned(),
            layer: layer.to_owned(),
            positions: self.positions,
            normals: self.normals,
            indices: self.indices,
        }
    }
}

#[wasm_bindgen]
pub fn parse_kicad_pcb(source: &str) -> Result<JsValue, JsValue> {
    let bundle = parse_board(source).map_err(|message| JsValue::from_str(&message))?;
    serde_wasm_bindgen::to_value(&bundle).map_err(|error| JsValue::from_str(&error.to_string()))
}

fn parse_board(source: &str) -> Result<GeometryBundle, String> {
    let root = parse_sexpr(source)?;
    let root_list = as_list(&root).ok_or("Invalid KiCad PCB document")?;
    if atom(root_list.first()).unwrap_or_default() != "kicad_pcb" {
        return Err("The selected file is not a KiCad PCB document".to_owned());
    }

    let thickness = child(root_list, "general")
        .and_then(|general| child(general, "thickness"))
        .and_then(|entry| number(entry.get(1)))
        .filter(|value| *value > 0.0)
        .unwrap_or(1.6);

    let layer_count = child(root_list, "layers")
        .map(|layers| {
            layers
                .iter()
                .skip(1)
                .filter(|entry| {
                    as_list(entry)
                        .and_then(|list| atom(list.get(1)))
                        .is_some_and(|name| name.ends_with(".Cu"))
                })
                .count()
        })
        .unwrap_or(2)
        .max(2);

    let mut outline_chains: Vec<Vec<Point2>> = Vec::new();
    let mut feature_points: Vec<Point2> = Vec::new();
    let mut traces = MeshBuilder::default();
    let mut pads = MeshBuilder::default();
    let mut vias = MeshBuilder::default();
    let mut drills = MeshBuilder::default();
    let mut silk = MeshBuilder::default();
    let mut component_count = 0;
    let mut pad_count = 0;
    let mut track_count = 0;
    let mut via_count = 0;
    let half = thickness * 0.5;

    for entry in root_list.iter().skip(1) {
        let Some(list) = as_list(entry) else { continue };
        match atom(list.first()).unwrap_or_default() {
            "gr_line" => {
                if let (Some(start), Some(end), Some(layer)) = (
                    point_child(list, "start"),
                    point_child(list, "end"),
                    child_atom(list, "layer"),
                ) {
                    if layer == "Edge.Cuts" {
                        outline_chains.push(vec![start, end]);
                    } else if layer == "F.SilkS" || layer == "B.SilkS" {
                        let y = if layer == "F.SilkS" {
                            half + SILK_THICKNESS
                        } else {
                            -half - SILK_THICKNESS
                        };
                        silk.add_segment_prism(
                            start,
                            end,
                            y,
                            stroke_width(list).max(0.12),
                            SILK_THICKNESS,
                        );
                    }
                }
            }
            "gr_rect" => {
                if let (Some(start), Some(end), Some(layer)) = (
                    point_child(list, "start"),
                    point_child(list, "end"),
                    child_atom(list, "layer"),
                ) {
                    let corners = vec![
                        start,
                        Point2 {
                            x: end.x,
                            y: start.y,
                        },
                        end,
                        Point2 {
                            x: start.x,
                            y: end.y,
                        },
                        start,
                    ];
                    if layer == "Edge.Cuts" {
                        outline_chains.push(corners);
                    } else if layer == "F.SilkS" || layer == "B.SilkS" {
                        let y = if layer == "F.SilkS" {
                            half + SILK_THICKNESS
                        } else {
                            -half - SILK_THICKNESS
                        };
                        for pair in corners.windows(2) {
                            silk.add_segment_prism(
                                pair[0],
                                pair[1],
                                y,
                                stroke_width(list).max(0.12),
                                SILK_THICKNESS,
                            );
                        }
                    }
                }
            }
            "gr_arc" => {
                if child_atom(list, "layer") == Some("Edge.Cuts") {
                    if let (Some(start), Some(mid), Some(end)) = (
                        point_child(list, "start"),
                        point_child(list, "mid"),
                        point_child(list, "end"),
                    ) {
                        outline_chains.push(tessellate_arc(start, mid, end));
                    }
                }
            }
            "gr_circle" => {
                if child_atom(list, "layer") == Some("Edge.Cuts") {
                    if let (Some(center), Some(end)) =
                        (point_child(list, "center"), point_child(list, "end"))
                    {
                        let radius = distance(center, end);
                        let mut circle = Vec::with_capacity(49);
                        for i in 0..=48 {
                            let angle = i as f32 * 2.0 * PI / 48.0;
                            circle.push(Point2 {
                                x: center.x + radius * angle.cos(),
                                y: center.y + radius * angle.sin(),
                            });
                        }
                        outline_chains.push(circle);
                    }
                }
            }
            "segment" => {
                if let (Some(start), Some(end)) =
                    (point_child(list, "start"), point_child(list, "end"))
                {
                    let width = child(list, "width")
                        .and_then(|v| number(v.get(1)))
                        .unwrap_or(0.2);
                    let layer = child_atom(list, "layer").unwrap_or("F.Cu");
                    let y = copper_layer_y(layer, thickness, layer_count);
                    traces.add_segment_prism(start, end, y, width.max(0.03), COPPER_THICKNESS);
                    feature_points.extend_from_slice(&[start, end]);
                    track_count += 1;
                }
            }
            "via" => {
                if let Some(at) = point_child(list, "at") {
                    let size = child(list, "size")
                        .and_then(|v| number(v.get(1)))
                        .unwrap_or(0.8);
                    let drill = child(list, "drill")
                        .and_then(first_numeric_value)
                        .unwrap_or(size * 0.5);
                    vias.add_cylinder(at, 0.0, size * 0.5, thickness + COPPER_THICKNESS * 2.0, 20);
                    drills.add_cylinder(
                        at,
                        0.0,
                        drill * 0.5,
                        thickness + COPPER_THICKNESS * 4.0,
                        20,
                    );
                    feature_points.push(at);
                    via_count += 1;
                }
            }
            "footprint" | "module" => {
                component_count += 1;
                parse_footprint(
                    list,
                    thickness,
                    layer_count,
                    &mut pads,
                    &mut vias,
                    &mut drills,
                    &mut silk,
                    &mut feature_points,
                    &mut pad_count,
                );
            }
            _ => {}
        }
    }

    let mut outline = chain_outline(outline_chains);
    if outline.len() < 3 {
        outline = fallback_outline(&feature_points);
    }
    if outline
        .first()
        .is_some_and(|first| outline.last().is_some_and(|last| close(*first, *last)))
    {
        outline.pop();
    }

    let (min, max) = bounds(&outline);
    let center = Point2 {
        x: (min.x + max.x) * 0.5,
        y: (min.y + max.y) * 0.5,
    };
    translate_outline(&mut outline, center);
    translate_builder(&mut traces, center);
    translate_builder(&mut pads, center);
    translate_builder(&mut vias, center);
    translate_builder(&mut drills, center);
    translate_builder(&mut silk, center);

    let board = build_board_mesh(&outline, thickness)?;
    let mut meshes = vec![board.finish("board", "board", "board")];
    push_mesh(&mut meshes, traces, "traces", "copper", "copper");
    push_mesh(&mut meshes, pads, "pads", "pad", "copper");
    push_mesh(&mut meshes, vias, "vias", "copper", "copper");
    push_mesh(&mut meshes, drills, "drills", "drill", "drill");
    push_mesh(&mut meshes, silk, "silkscreen", "silk", "silkscreen");

    Ok(GeometryBundle {
        meshes,
        stats: BoardStats {
            components: component_count,
            pads: pad_count,
            tracks: track_count,
            vias: via_count,
            layers: layer_count,
        },
        bounds: BoardBounds {
            width: max.x - min.x,
            height: max.y - min.y,
            thickness,
        },
    })
}

#[allow(clippy::too_many_arguments)]
fn parse_footprint(
    list: &[SExpr],
    thickness: f32,
    layer_count: usize,
    pads: &mut MeshBuilder,
    barrels: &mut MeshBuilder,
    drills: &mut MeshBuilder,
    silk: &mut MeshBuilder,
    feature_points: &mut Vec<Point2>,
    pad_count: &mut usize,
) {
    let footprint_at = point_child(list, "at").unwrap_or_default();
    let footprint_rotation = child(list, "at")
        .and_then(|at| number(at.get(3)))
        .unwrap_or(0.0);

    for entry in list.iter().skip(1) {
        let Some(item) = as_list(entry) else { continue };
        match atom(item.first()).unwrap_or_default() {
            "pad" => {
                let pad_type = atom(item.get(2)).unwrap_or("smd");
                let shape = atom(item.get(3)).unwrap_or("rect");
                let local = point_child(item, "at").unwrap_or_default();
                let local_rotation = child(item, "at")
                    .and_then(|at| number(at.get(3)))
                    .unwrap_or(0.0);
                let at = transform_point(local, footprint_at, footprint_rotation);
                let size = point_child(item, "size").unwrap_or(Point2 { x: 1.0, y: 1.0 });
                let layers = child(item, "layers")
                    .map(|entry| {
                        entry
                            .iter()
                            .skip(1)
                            .filter_map(|node| atom(Some(node)))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let total_rotation = footprint_rotation + local_rotation;
                feature_points.push(at);
                *pad_count += 1;

                if pad_type == "smd" || pad_type == "connect" {
                    let board_layer = layers
                        .iter()
                        .find(|layer| layer.ends_with(".Cu"))
                        .copied()
                        .unwrap_or("F.Cu");
                    let y = copper_layer_y(board_layer, thickness, layer_count);
                    add_pad_shape(pads, shape, at, y, size, COPPER_THICKNESS, total_rotation);
                } else {
                    let outer = size.x.max(size.y) * 0.5;
                    let drill = child(item, "drill")
                        .and_then(first_numeric_value)
                        .unwrap_or(size.x.min(size.y) * 0.5);
                    barrels.add_cylinder(at, 0.0, outer, thickness + COPPER_THICKNESS * 2.0, 20);
                    drills.add_cylinder(
                        at,
                        0.0,
                        drill * 0.5,
                        thickness + COPPER_THICKNESS * 4.0,
                        20,
                    );
                }
            }
            "fp_line" => {
                let layer = child_atom(item, "layer").unwrap_or_default();
                if layer == "F.SilkS" || layer == "B.SilkS" {
                    if let (Some(start), Some(end)) =
                        (point_child(item, "start"), point_child(item, "end"))
                    {
                        let start = transform_point(start, footprint_at, footprint_rotation);
                        let end = transform_point(end, footprint_at, footprint_rotation);
                        let y = if layer == "F.SilkS" {
                            thickness * 0.5 + SILK_THICKNESS
                        } else {
                            -thickness * 0.5 - SILK_THICKNESS
                        };
                        silk.add_segment_prism(
                            start,
                            end,
                            y,
                            stroke_width(item).max(0.12),
                            SILK_THICKNESS,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

fn add_pad_shape(
    builder: &mut MeshBuilder,
    shape: &str,
    at: Point2,
    y: f32,
    size: Point2,
    height: f32,
    rotation: f32,
) {
    match shape {
        "circle" => builder.add_cylinder(at, y, size.x * 0.5, height, 24),
        "oval" => builder.add_oriented_box(at, y, size.x, size.y, height, rotation),
        _ => builder.add_oriented_box(at, y, size.x, size.y, height, rotation),
    }
}

fn build_board_mesh(outline: &[Point2], thickness: f32) -> Result<MeshBuilder, String> {
    if outline.len() < 3 {
        return Err("A board outline could not be derived from the file".to_owned());
    }
    let mut builder = MeshBuilder::default();
    let coordinates: Vec<f64> = outline
        .iter()
        .flat_map(|point| [point.x as f64, (-point.y) as f64])
        .collect();
    let triangles = earcutr::earcut(&coordinates, &[], 2)
        .map_err(|error| format!("Unable to triangulate board outline: {error}"))?;
    let half = thickness * 0.5;

    for triangle in triangles.chunks_exact(3) {
        let a = outline[triangle[0]];
        let b = outline[triangle[1]];
        let c = outline[triangle[2]];
        builder.triangle(
            to_three(a, half),
            to_three(b, half),
            to_three(c, half),
            Point3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        );
        builder.triangle(
            to_three(a, -half),
            to_three(c, -half),
            to_three(b, -half),
            Point3 {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        );
    }

    let area = polygon_area_three(outline);
    for i in 0..outline.len() {
        let a = outline[i];
        let b = outline[(i + 1) % outline.len()];
        let dx = b.x - a.x;
        let dz = -(b.y - a.y);
        let length = (dx * dx + dz * dz).sqrt().max(f32::EPSILON);
        let normal = if area > 0.0 {
            Point3 {
                x: dz / length,
                y: 0.0,
                z: -dx / length,
            }
        } else {
            Point3 {
                x: -dz / length,
                y: 0.0,
                z: dx / length,
            }
        };
        builder.quad(
            to_three(a, -half),
            to_three(b, -half),
            to_three(b, half),
            to_three(a, half),
            normal,
        );
    }
    Ok(builder)
}

fn tokenize(input: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = input.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '(' | ')' => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                tokens.push(ch.to_string());
            }
            '"' => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                let mut value = String::new();
                let mut closed = false;
                while let Some(inner) = chars.next() {
                    match inner {
                        '\\' => {
                            if let Some(escaped) = chars.next() {
                                value.push(escaped);
                            }
                        }
                        '"' => {
                            closed = true;
                            break;
                        }
                        _ => value.push(inner),
                    }
                }
                if !closed {
                    return Err("Unterminated string in KiCad file".to_owned());
                }
                tokens.push(value);
            }
            value if value.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

fn parse_sexpr(input: &str) -> Result<SExpr, String> {
    let tokens = tokenize(input)?;
    let mut stack: Vec<Vec<SExpr>> = Vec::new();
    let mut root: Option<SExpr> = None;
    for token in tokens {
        match token.as_str() {
            "(" => stack.push(Vec::new()),
            ")" => {
                let list = stack.pop().ok_or("Unexpected closing parenthesis")?;
                let node = SExpr::List(list);
                if let Some(parent) = stack.last_mut() {
                    parent.push(node);
                } else if root.replace(node).is_some() {
                    return Err("Multiple root expressions in KiCad file".to_owned());
                }
            }
            _ => {
                let current = stack
                    .last_mut()
                    .ok_or("Token outside the KiCad root expression")?;
                current.push(SExpr::Atom(token));
            }
        }
    }
    if !stack.is_empty() {
        return Err("Unclosed parenthesis in KiCad file".to_owned());
    }
    root.ok_or_else(|| "Empty KiCad file".to_owned())
}

fn as_list(node: &SExpr) -> Option<&[SExpr]> {
    match node {
        SExpr::List(list) => Some(list),
        _ => None,
    }
}

fn atom(node: Option<&SExpr>) -> Option<&str> {
    match node {
        Some(SExpr::Atom(value)) => Some(value),
        _ => None,
    }
}

fn number(node: Option<&SExpr>) -> Option<f32> {
    atom(node)?.parse().ok()
}

fn child<'a>(list: &'a [SExpr], key: &str) -> Option<&'a [SExpr]> {
    list.iter().find_map(|node| {
        let candidate = as_list(node)?;
        (atom(candidate.first()) == Some(key)).then_some(candidate)
    })
}

fn child_atom<'a>(list: &'a [SExpr], key: &str) -> Option<&'a str> {
    child(list, key).and_then(|entry| atom(entry.get(1)))
}

fn point_child(list: &[SExpr], key: &str) -> Option<Point2> {
    let entry = child(list, key)?;
    Some(Point2 {
        x: number(entry.get(1))?,
        y: number(entry.get(2))?,
    })
}

fn first_numeric_value(list: &[SExpr]) -> Option<f32> {
    list.iter().skip(1).find_map(|entry| number(Some(entry)))
}

fn stroke_width(list: &[SExpr]) -> f32 {
    child(list, "stroke")
        .and_then(|stroke| child(stroke, "width"))
        .and_then(|width| number(width.get(1)))
        .or_else(|| child(list, "width").and_then(|width| number(width.get(1))))
        .unwrap_or(0.15)
}

fn copper_layer_y(layer: &str, thickness: f32, layer_count: usize) -> f32 {
    let half = thickness * 0.5;
    if layer == "F.Cu" {
        return half + COPPER_THICKNESS * 0.5;
    }
    if layer == "B.Cu" {
        return -half - COPPER_THICKNESS * 0.5;
    }
    if let Some(index) = layer
        .strip_prefix("In")
        .and_then(|value| value.strip_suffix(".Cu"))
        .and_then(|value| value.parse::<usize>().ok())
    {
        let inner_count = layer_count.saturating_sub(2);
        if inner_count > 0 {
            return half - index as f32 / (inner_count + 1) as f32 * thickness;
        }
    }
    0.0
}

fn transform_point(local: Point2, origin: Point2, rotation_deg: f32) -> Point2 {
    let angle = rotation_deg.to_radians();
    Point2 {
        x: origin.x + local.x * angle.cos() - local.y * angle.sin(),
        y: origin.y + local.x * angle.sin() + local.y * angle.cos(),
    }
}

fn tessellate_arc(start: Point2, mid: Point2, end: Point2) -> Vec<Point2> {
    let determinant =
        2.0 * (start.x * (mid.y - end.y) + mid.x * (end.y - start.y) + end.x * (start.y - mid.y));
    if determinant.abs() < 1e-6 {
        return vec![start, end];
    }
    let s2 = start.x * start.x + start.y * start.y;
    let m2 = mid.x * mid.x + mid.y * mid.y;
    let e2 = end.x * end.x + end.y * end.y;
    let center = Point2 {
        x: (s2 * (mid.y - end.y) + m2 * (end.y - start.y) + e2 * (start.y - mid.y)) / determinant,
        y: (s2 * (end.x - mid.x) + m2 * (start.x - end.x) + e2 * (mid.x - start.x)) / determinant,
    };
    let start_angle = (start.y - center.y).atan2(start.x - center.x);
    let mid_angle = (mid.y - center.y).atan2(mid.x - center.x);
    let end_angle = (end.y - center.y).atan2(end.x - center.x);
    let ccw_end = positive_angle(end_angle - start_angle);
    let ccw_mid = positive_angle(mid_angle - start_angle);
    let sweep = if ccw_mid <= ccw_end {
        ccw_end
    } else {
        ccw_end - 2.0 * PI
    };
    let radius = distance(center, start);
    let steps = ((sweep.abs() / (PI / 24.0)).ceil() as usize).max(2);
    (0..=steps)
        .map(|index| {
            let angle = start_angle + sweep * index as f32 / steps as f32;
            Point2 {
                x: center.x + radius * angle.cos(),
                y: center.y + radius * angle.sin(),
            }
        })
        .collect()
}

fn positive_angle(value: f32) -> f32 {
    value.rem_euclid(2.0 * PI)
}

fn chain_outline(mut chains: Vec<Vec<Point2>>) -> Vec<Point2> {
    chains.retain(|chain| chain.len() >= 2);
    if chains.is_empty() {
        return Vec::new();
    }
    chains.sort_by(|a, b| polyline_length(b).total_cmp(&polyline_length(a)));
    let mut outline = chains.remove(0);
    while let Some(current) = outline.last().copied() {
        if outline.len() > 2 && close(current, outline[0]) {
            break;
        }
        let match_index = chains
            .iter()
            .position(|chain| close(current, chain[0]) || close(current, *chain.last().unwrap()));
        let Some(index) = match_index else { break };
        let mut chain = chains.remove(index);
        if !close(current, chain[0]) {
            chain.reverse();
        }
        outline.extend(chain.into_iter().skip(1));
    }
    outline
}

fn fallback_outline(points: &[Point2]) -> Vec<Point2> {
    let (mut min, mut max) = if points.is_empty() {
        (Point2 { x: -25.0, y: -20.0 }, Point2 { x: 25.0, y: 20.0 })
    } else {
        bounds(points)
    };
    min.x -= 4.0;
    min.y -= 4.0;
    max.x += 4.0;
    max.y += 4.0;
    vec![
        min,
        Point2 { x: max.x, y: min.y },
        max,
        Point2 { x: min.x, y: max.y },
    ]
}

fn bounds(points: &[Point2]) -> (Point2, Point2) {
    let mut min = Point2 {
        x: f32::INFINITY,
        y: f32::INFINITY,
    };
    let mut max = Point2 {
        x: f32::NEG_INFINITY,
        y: f32::NEG_INFINITY,
    };
    for point in points {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    (min, max)
}

fn translate_outline(points: &mut [Point2], center: Point2) {
    for point in points {
        point.x -= center.x;
        point.y -= center.y;
    }
}

fn translate_builder(builder: &mut MeshBuilder, center: Point2) {
    for vertex in builder.positions.chunks_exact_mut(3) {
        vertex[0] -= center.x;
        vertex[2] += center.y;
    }
}

fn push_mesh(
    meshes: &mut Vec<MeshData>,
    builder: MeshBuilder,
    name: &str,
    material: &str,
    layer: &str,
) {
    if !builder.is_empty() {
        meshes.push(builder.finish(name, material, layer));
    }
}

fn to_three(point: Point2, y: f32) -> Point3 {
    Point3 {
        x: point.x,
        y,
        z: -point.y,
    }
}
fn distance(a: Point2, b: Point2) -> f32 {
    ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt()
}
fn close(a: Point2, b: Point2) -> bool {
    distance(a, b) <= EPSILON
}
fn polyline_length(points: &[Point2]) -> f32 {
    points
        .windows(2)
        .map(|pair| distance(pair[0], pair[1]))
        .sum()
}
fn polygon_area_three(points: &[Point2]) -> f32 {
    (0..points.len())
        .map(|i| {
            let a = Point2 {
                x: points[i].x,
                y: -points[i].y,
            };
            let b = Point2 {
                x: points[(i + 1) % points.len()].x,
                y: -points[(i + 1) % points.len()].y,
            };
            a.x * b.y - b.x * a.y
        })
        .sum::<f32>()
        * 0.5
}
fn sub3(a: Point3, b: Point3) -> Point3 {
    Point3 {
        x: a.x - b.x,
        y: a.y - b.y,
        z: a.z - b.z,
    }
}
fn cross3(a: Point3, b: Point3) -> Point3 {
    Point3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}
fn dot3(a: Point3, b: Point3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOARD: &str = r#"
      (kicad_pcb (version 20240108)
        (general (thickness 1.6))
        (layers (0 "F.Cu" signal) (31 "B.Cu" signal) (44 "Edge.Cuts" user))
        (gr_rect (start 0 0) (end 40 30) (stroke (width 0.05) (type default)) (fill none) (layer "Edge.Cuts"))
        (footprint "Package_QFP:LQFP-48" (layer "F.Cu") (at 20 15 90)
          (pad "1" smd roundrect (at 2 0 90) (size 0.5 1.5) (layers "F.Cu" "F.Paste" "F.Mask"))
          (pad "2" thru_hole circle (at -2 0) (size 1.8 1.8) (drill 0.9) (layers "*.Cu" "*.Mask")))
        (segment (start 5 5) (end 20 15) (width 0.25) (layer "F.Cu") (net 1))
        (via (at 10 10) (size 0.8) (drill 0.4) (layers "F.Cu" "B.Cu") (net 1)))
    "#;

    #[test]
    fn parses_board_into_mesh_buffers() {
        let bundle = parse_board(BOARD).expect("board should parse");
        assert_eq!(bundle.stats.components, 1);
        assert_eq!(bundle.stats.pads, 2);
        assert_eq!(bundle.stats.tracks, 1);
        assert_eq!(bundle.stats.vias, 1);
        assert!((bundle.bounds.width - 40.0).abs() < 0.01);
        assert!(bundle.meshes.iter().all(|mesh| !mesh.positions.is_empty()));
    }

    #[test]
    fn rejects_non_kicad_input() {
        assert!(parse_board("(not_a_board)").is_err());
    }
}
