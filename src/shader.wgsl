struct Uniforms {
	resolution: vec2f,
	center: vec2f,
	scale: f32,
	max_iter: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexIn {
	@builtin(vertex_index) vertex_index: u32,
}

struct VertexOut {
	@builtin(position) position: vec4f,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
	let uv = vec2f(vec2u((in.vertex_index << 1) & 2, in.vertex_index & 2));
	let position = vec4f(uv * 2. - 1., 0., 1.);
	return VertexOut(position);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
	let p0 = uniforms.center + (in.position.xy - uniforms.resolution * .5) * uniforms.scale;
	var p = p0;
	var i: u32 = 0;
	for (; i < uniforms.max_iter; i = i + 1) {
		let d = p * p;
		if (d.x + d.y > 4.) {
			break;
		}

		p = vec2f(d.x - d.y + p0.x, 2. * p.x * p.y + p0.y);
	}

	if (i >= uniforms.max_iter) {
		return vec4f(vec3f(0.), 1.);
	} else {
		return vec4f(vec3f(f32(i) / f32(uniforms.max_iter)), 1.);
	}
}
