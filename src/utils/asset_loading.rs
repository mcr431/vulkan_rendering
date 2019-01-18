extern crate tobj;

use std::path::Path;
use crate::primitives::vertex::Vertex;
use crate::primitives::geometry::Geometry;

pub fn load_model(path: &Path) -> Geometry<Vertex> {
    use tobj::{load_obj};

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    println!("Starting to load model: {:?}", path);
    let obj = load_obj(path);
    println!("Finishing model load: {:?}", path);

    let (models, materials) = obj.unwrap();

    for (i, m) in models.iter().enumerate() {
        let mesh = &m.mesh;

        for index in &mesh.indices {
            let ind_usize = *index as usize;
            let pos = [
                mesh.positions[ind_usize * 3 + 0],
                mesh.positions[ind_usize * 3 + 1],
                mesh.positions[ind_usize * 3 + 2],
            ];

            let color = [1.0, 1.0, 1.0];

            let tex_coord = [
                mesh.texcoords[ind_usize * 2],
                1.0 - mesh.texcoords[ind_usize * 2 + 1],
            ];

            let vertex = Vertex::new(pos, color, tex_coord);
            vertices.push(vertex);
            indices.push(indices.len() as u32);
        }
    }

    Geometry::new(vertices, indices)
}

//pub fn load_image() -> {
//
//}