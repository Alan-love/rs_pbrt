use pbrt::core::transform::Transform;

fn main() {
    let t: Transform = Transform::scale(2.0, -8.0, 1.0);

    println!("t = {:?}", t);
}
