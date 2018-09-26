// pbrt
use core::pbrt::find_interval;
use core::pbrt::Float;

/// Calculates an offset and four weights for Catmull-Rom spline
/// interpolation.
pub fn catmull_rom_weights(
    nodes: &Vec<Float>,
    x: Float,
    offset: &mut i32,
    weights: &mut [Float; 4],
) -> bool {
    // return _false_ if _x_ is out of bounds
    if !(x >= *nodes.first().unwrap() && x <= *nodes.last().unwrap()) {
        return false;
    }
    // search for the interval _idx_ containing _x_
    let idx: usize = find_interval(nodes.len(), |index| nodes[index as usize] <= x);
    *offset = idx as i32 - 1;
    let x0: Float = nodes[idx];
    let x1: Float = nodes[idx + 1];
    // compute the $t$ parameter and powers
    let t: Float = (x - x0) / (x1 - x0);
    let t2: Float = t * t;
    let t3: Float = t2 * t;
    // compute initial node weights $w_1$ and $w_2$
    weights[1] = 2.0 as Float * t3 - 3.0 as Float * t2 + 1.0 as Float;
    weights[2] = -2.0 as Float * t3 + 3.0 as Float * t2;
    // compute first node weight $w_0$
    if idx > 0_usize {
        let w0: Float = (t3 - 2.0 as Float * t2 + t) * (x1 - x0) / (x1 - nodes[idx - 1]);
        weights[0] = -w0;
        weights[2] += w0;
    } else {
        let w0: Float = t3 - 2.0 as Float * t2 + t;
        weights[0] = 0.0 as Float;
        weights[1] -= w0;
        weights[2] += w0;
    }
    // compute last node weight $w_3$
    if (idx + 2) < nodes.len() {
        let w3: Float = (t3 - t2) * (x1 - x0) / (nodes[idx + 2] - x0);
        weights[1] -= w3;
        weights[3] = w3;
    } else {
        let w3: Float = t3 - t2;
        weights[1] -= w3;
        weights[2] += w3;
        weights[3] = 0.0 as Float;
    }
    true
}

pub fn sample_catmull_rom_2d(
    nodes1: &Vec<Float>,
    nodes2: &Vec<Float>,
    values: &Vec<Float>,
    cdf: &Vec<Float>,
    alpha: Float,
    u: Float,
    fval: Option<&mut Float>,
    pdf: Option<&mut Float>,
) -> Float {
    let size1 = nodes1.len();
    let size2 = nodes2.len();
    // local copy
    let mut u: Float = u;
    // determine offset and coefficients for the _alpha_ parameter
    let mut offset: i32 = 0;
    let mut weights: [Float; 4] = [0.0 as Float; 4];
    if !catmull_rom_weights(nodes1, alpha, &mut offset, &mut weights) {
        return 0.0 as Float;
    }
    // define a lambda function to interpolate table entries
    let interpolate = |array: &Vec<Float>, idx: usize| -> Float {
        let mut value: Float = 0.0;
        for i in 0..4 {
            if weights[i] != 0.0 as Float {
                value += array[(offset as usize + i) * size2 + idx] * weights[i];
            }
        }
        value
    };
    // map _u_ to a spline interval by inverting the interpolated _cdf_
    let maximum: Float = interpolate(cdf, size2 - 1_usize);
    u *= maximum;
    let idx: usize = find_interval(size2, |index| interpolate(cdf, index) <= u);
    // look up node positions and interpolated function values
    let f0: Float = interpolate(values, idx);
    let f1: Float = interpolate(values, idx + 1);
    let x0: Float = nodes2[idx];
    let x1: Float = nodes2[idx + 1];
    let width: Float = x1 - x0;
    // re-scale _u_ using the interpolated _cdf_
    u = (u - interpolate(cdf, idx)) / width;
    // approximate derivatives using finite differences of the interpolant
    let d0: Float;
    let d1: Float;
    if idx > 0_usize {
        d0 = width * (f1 - interpolate(values, idx - 1)) / (x1 - nodes2[idx - 1]);
    } else {
        d0 = f1 - f0;
    }
    if (idx + 2) < size2 {
        d1 = width * (interpolate(values, idx + 2) - f0) / (nodes2[idx + 2] - x0);
    } else {
        d1 = f1 - f0;
    }

    // invert definite integral over spline segment and return solution

    // set initial guess for $t$ by importance sampling a linear interpolant
    let mut t: Float;
    if f0 != f1 {
        t = (f0
            - (0.0 as Float)
                .max(f0 * f0 + 2.0 as Float * u * (f1 - f0))
                .sqrt())
            / (f0 - f1);
    } else {
        t = u / f0;
    }
    let mut a: Float = 0.0;
    let mut b: Float = 1.0;
    let mut f_hat: Float = 0.0;
    let mut fhat: Float = 0.0;
    loop {
        // fall back to a bisection step when _t_ is out of bounds
        if !(t >= a && t <= b) {
            t = 0.5 as Float * (a + b);
        }
        // evaluate target function and its derivative in Horner form
        f_hat = t
            * (f0
                + t * (0.5 as Float * d0
                    + t * ((1.0 as Float / 3.0 as Float) * (-2.0 as Float * d0 - d1) + f1 - f0
                        + t * (0.25 as Float * (d0 + d1) + 0.5 as Float * (f0 - f1)))));
        fhat = f0
            + t * (d0
                + t * (-2.0 as Float * d0 - d1
                    + 3.0 as Float * (f1 - f0)
                    + t * (d0 + d1 + 2.0 as Float * (f0 - f1))));
        // stop the iteration if converged
        if (f_hat - u).abs() < 1e-6 as Float || b - a < 1e-6 as Float {
            break;
        }
        // update bisection bounds using updated _t_
        if (f_hat - u) < 0.0 as Float {
            a = t;
        } else {
            b = t;
        }
        // perform a Newton step
        t -= (f_hat - u) / fhat;
    }
    // return the sample position and function value
    if let Some(fval) = fval {
        *fval = fhat;
    }
    if let Some(pdf) = pdf {
        *pdf = fhat / maximum;
    }
    x0 + width * t
}

/// Evaluates the weighted sum of cosines.
pub fn fourier(a: &Vec<Float>, si: usize, m: i32, cos_phi: f64) -> Float {
    let mut value: f64 = 0.0;
    // initialize cosine iterates
    let mut cos_k_minus_one_phi: f64 = cos_phi;
    let mut cos_k_phi: f64 = 1.0;
    for k in 0..m as usize {
        // add the current summand and update the cosine iterates
        value += a[si + k] as f64 * cos_k_phi;
        let cos_k_plus_one_phi: f64 = 2.0 as f64 * cos_phi * cos_k_phi - cos_k_minus_one_phi;
        cos_k_minus_one_phi = cos_k_phi;
        cos_k_phi = cos_k_plus_one_phi;
    }
    value as Float
}