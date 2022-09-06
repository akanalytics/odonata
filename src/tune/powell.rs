use ndarray::prelude::*;
// use num_traits::Float;

pub struct Powell {
    pub n: usize,
    pub max_iter: i32,
    pub ε: f32,
    pub verbose: bool,
    pub x0: Array1<f32>,
}
//
// https://home.cc.umanitoba.ca/~lovetrij/cECE7670/
//
// In words, Powell’s method to minimize a function f( ) x in Rn

// can be described as

// follows.
// • First, initialize n search directions si

// , i = 1, ...n to the coordinate unit vectors

// ei
// , i = 1, ...n .

// • Then, starting at an initial guess, x
// 0

// , perform and initial search in the sn direc-
// tion which gets you to the point X.

// • Store X in Y and then update X by performing n successive minimizations
// along the n search directions.
// • Create a new search direction, sn + 1 = X Y– and minimize along this direction
// as well.
// • After this last search we check for convergence by comparing the relative change
// in function value at the most recent X with respect to the function value at Y.
// • If we have not converged, then we discard the first search direction s1 and let
// si si + 1 = , i = 1, ...n and repeat.

// Algorithm: Powell’s Method
// 1. input: f(x) , x0 , ε , max_iteration
// 2. set: s[i] = e[i] = , i = 1, ...n
// 3. find λ which minimizes f(x0 + λ * s[n]) +
// 4. set: X = x0 + λ* s[n] C=false, k = 0
// 5. while C ≡ False repeat
// 6.   set: Y = X , k = k + 1
// 7.    for i = 1..n
// 8.       find λ which minimizes f(X + λs[i])
// 9.       set: X = X + λs[i]
// 10.   end
// 11.   set: s[i+1] = X - Y
// 12.   find λ which minimizes f(X+λs[i+1])
// 13.   set: X = X + λs[i+1]
// 14.   if k > max_iteration OR |f(X) – f(Y)| / max( f(X), 10e–10] < ε
// 15.      C = True
// 16.   else
// 17.      for i = 1..n
// 18.         set si = s[i + 1]
// 19.       end
// 20. end
// 21. end

pub struct Solver1D {
    max_iter: i32,
    ε: f32,
    verbose: bool,
    pub x0: f32,
    x_min: f32,
    x_max: f32,
}

// algo from https://github.com/scijs/minimize-golden-section-1d
// license: MIT
impl Solver1D {
    pub fn minimize(&self, mut f: impl FnMut(f32) -> f32) -> f32 {
        #[allow(non_snake_case)]
        let PHI_RATIO: f32 = 2.0 / (1.0 + f32::sqrt(5.0));
        let ε = self.ε;

        let mut x_min = self.x_min;
        let mut x_max = self.x_max;
        let x_mid;
        let f_mid;
        let mut iter = 0;
        let mut x1 = x_max - PHI_RATIO * (x_max - x_min);
        let mut x2 = x_min + PHI_RATIO * (x_max - x_min);

        // Initial bounds:
        let mut f1 = f(x1);
        let mut f2 = f(x2);

        // Store these values so that we can return these if they're better.
        // This happens when the minimization falls *approaches* but never
        // actually reaches one of the bounds
        let f10 = f(x_min);
        let f20 = f(x_max);

        while iter < self.max_iter && f32::abs(x_max - x_min) > ε {
            iter += 1;
            if f2 > f1 {
                x_max = x2;
                x2 = x1;
                f2 = f1;
                x1 = x_max - PHI_RATIO * (x_max - x_min);
                f1 = f(x1);
                if self.verbose {
                    println!(".{iter} f({x1}) = {f1}");
                }
                } else {
                x_min = x1;
                x1 = x2;
                f1 = f2;
                x2 = x_min + PHI_RATIO * (x_max - x_min);
                f2 = f(x2);
                if self.verbose {
                    println!(".{iter} f({x2}) = {f2}");
                }
                }
        }

        x_mid = 0.5 * (x_max + x_min);
        f_mid = 0.5 * (f1 + f2);

        //   if (status) {
        //     status.iterations = iteration;
        //     status.argmin = xF;
        //     status.minimum = fF;
        //     status.converged = true;
        //   }

        if !f2.is_finite() || !f1.is_finite() || iter == self.max_iter {
            // if (status) {
            //   status.converged = false;
            // }
            if self.verbose {
                let ε = f32::abs(x_max - x_min);
                println!("Failed iters = {iter}  f1 = {f1} f2 = {f2} x_min = {x_min} x_max = {x_max} ε = {ε} ");
            }
            return f32::NAN;
        }

        let ans = match f_mid {
            f_mid if f_mid > f10 => self.x_min,
            f_mid if f_mid > f20 => self.x_max,
            _ => x_mid,
        };
        if self.verbose {
            println!("1D minimise λ = {ans}");
        }
        ans
    }
}

impl Powell {
    pub fn minimize(&self, f: &mut impl FnMut(ArrayView1<f32>) -> f32) -> Array1<f32> {
        let max_iter = self.max_iter;
        let ε = self.ε;
        let verbose = self.verbose;

        let solver_1d = Solver1D {
            max_iter,
            ε,
            verbose,
            x0: 0.0,
            x_min: -100.0,
            x_max: 4000.0,
        };
        let n = self.n;
        let x0 = &self.x0;
        let mut s = vec![Array::zeros(n); n + 1];

        // step 2
        for i in 0..n {
            s[i][i] = 1.0;
        }

        // step 3
        let f1 = |λ: f32| f((x0 + λ * &s[n-1]).view());
        let λ = solver_1d.minimize(f1);

        // step 4
        let mut x = x0 + λ * &s[n];
        let mut converged = false;
        let mut k = 0;

        while !converged {
            let y = x.clone();
            k += 1;
            for i in 0..n {
                let f1 = |λ: f32| f( (&x + λ * &s[i]).view() );
                let λ = solver_1d.minimize(f1);
                x += &(λ * &s[i]);
                if self.verbose {
                    println!("Iter {k} i = {i} x = {x}");
                }
    
            }
            s[n] = &x - &y;            

            // normalize s[n]
            let l2 = s[n].dot(&s[n]).sqrt();
            s[n].map_inplace(|x| *x /= l2);

            let f1 = |λ: f32| f( (&x + λ * &s[n]).view());
            let solver_1d_lam = Solver1D {
                max_iter,
                ε,
                verbose,
                x0: 0.5,
                x_min: 0.0,
                x_max: 1.0,
            };
    
            let λ = solver_1d_lam.minimize(f1);
            // if self.verbose {
                println!("Iter {k} s[n] = {}  x = {x} y = {y} new x = {}", s[n], &x + λ * &s[n]);
            // }
            x += &(λ * &s[n]);
            let fx = f(x.view());
            let fy = f(y.view());
            let err = (fx - fy).abs() / fx.max(1e-10);
            if k > self.max_iter || err < self.ε {
                converged = true;
            } else {
                for i in 0..n {
                    s[i] = s[i + 1].clone();
                }
            }
            if self.verbose {
                println!("Iter {k} x = {x} err = {err}");
            }
        }
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1d() {
        let solver = Solver1D {
            max_iter: 100,
            ε: 1e-6,
            verbose: true,
            x0: 0.0,
            x_min: -10.0,
            x_max: 10.0,
        };

        let x = solver.minimize(|x| (x - 3.0) * (x - 3.0));
        assert!(f32::abs(x - 3.0) < 1e-4);

        let x = solver.minimize(|x| f32::abs(x - 5.0));
        assert!(f32::abs(x - 5.0) < 1e-4);
    }

    #[test]
    fn test_2d() {
        let solver = Powell {
            n: 2,
            max_iter: 100,
            ε: 1e-6,
            verbose: true,
            x0: array![0.0, 0.0],
            // x_min: [-10.0, -10.0],
            // x_max: [10.0, 10.0],
        };

        let x = solver.minimize(&mut|x| (x[0] - 3.0).abs() + (x[1] - 4.0).abs());
        println!("x = {x}");

    }
}

// function powellsMethod (f, x0, options, status) {
//     var i, j, iter, ui, tmin, pj, fi, un, u, p0, sum, dx, err, perr, du, tlimit;

//     options = options || {};
//     var maxIter = options.maxIter === undefined ? 20 : options.maxIter;
//     var tol = options.tolerance === undefined ? 1e-8 : options.tolerance;
//     var tol1d = options.lineTolerance === undefined ? tol : options.lineTolerance;
//     var bounds = options.bounds === undefined ? [] : options.bounds;
//     var verbose = options.verbose === undefined ? false : options.verbose;

//     if (status) status.points = [];

//     // Dimensionality:
//     var n = x0.length;
//     // Solution vector:
//     var p = x0.slice(0);

//     // Search directions:
//     u = [];
//     un = [];
//     for (i = 0; i < n; i++) {
//       u[i] = [];
//       for (j = 0; j < n; j++) {
//         u[i][j] = i === j ? 1 : 0;
//       }
//     }

//     // Bound the input:
//     function constrain (x) {
//       for (var i = 0; i < bounds.length; i++) {
//         var ibounds = bounds[i];
//         if (!ibounds) continue;
//         if (isFinite(ibounds[0])) {
//           x[i] = Math.max(ibounds[0], x[i]);
//         }
//         if (isFinite(ibounds[1])) {
//           x[i] = Math.min(ibounds[1], x[i]);
//         }
//       }
//     }

//     constrain(p);

//     if (status) status.points.push(p.slice());

//     var bound = options.bounds
//       ? function (p, ui) {
//         var upper = Infinity;
//         var lower = -Infinity;

//         for (var j = 0; j < n; j++) {
//           var jbounds = bounds[j];
//           if (!jbounds) continue;

//           if (ui[j] !== 0) {
//             if (jbounds[0] !== undefined && isFinite(jbounds[0])) {
//               lower = (ui[j] > 0 ? Math.max : Math.min)(lower, (jbounds[0] - p[j]) / ui[j]);
//             }

//             if (jbounds[1] !== undefined && isFinite(jbounds[1])) {
//               upper = (ui[j] > 0 ? Math.min : Math.max)(upper, (jbounds[1] - p[j]) / ui[j]);
//             }
//           }
//         }

//         return [lower, upper];
//       }
//       : function () {
//         return [-Infinity, Infinity];
//       };

//     // A function to evaluate:
//     pj = [];
//     fi = function (t) {
//       for (var i = 0; i < n; i++) {
//         pj[i] = p[i] + ui[i] * t;
//       }

//       return f(pj);
//     };

//     iter = 0;
//     perr = 0;
//     while (++iter < maxIter) {
//       // Reinitialize the search vectors:
//       if (iter % (n) === 0) {
//         for (i = 0; i < n; i++) {
//           u[i] = [];
//           for (j = 0; j < n; j++) {
//             u[i][j] = i === j ? 1 : 0;
//           }
//         }
//       }

//       // Store the starting point p0:
//       for (j = 0, p0 = []; j < n; j++) {
//         p0[j] = p[j];
//       }

//       // Minimize over each search direction u[i]:
//       for (i = 0; i < n; i++) {
//         ui = u[i];

//         // Compute bounds based on starting point p in the
//         // direction ui:

//         tlimit = bound(p, ui);

//         // Minimize using golden section method:
//         dx = 0.1;

//         tmin = minimize1d(fi, {
//           lowerBound: tlimit[0],
//           upperBound: tlimit[1],
//           initialIncrement: dx,
//           tolerance: dx * tol1d
//         });

//         if (tmin === 0) {
//           return p;
//         }

//         // Update the solution vector:
//         for (j = 0; j < n; j++) {
//           p[j] += tmin * ui[j];
//         }

//         constrain(p);

//         if (status) status.points.push(p.slice());
//       }

//       // Throw out the first search direction:
//       u.shift();

//       // Construct a new search direction:
//       for (j = 0, un = [], sum = 0; j < n; j++) {
//         un[j] = p[j] - p0[j];
//         sum += un[j] * un[j];
//       }
//       // Normalize:
//       sum = Math.sqrt(sum);

//       if (sum > 0) {
//         for (j = 0; j < n; j++) {
//           un[j] /= sum;
//         }
//       } else {
//         // Exactly nothing moved, so it it appears we've converged. In particular,
//         // it's possible the solution is up against a boundary and simply can't
//         // move farther.
//         return p;
//       }

//       u.push(un);
//       // One more minimization, this time along the new direction:
//       ui = un;

//       tlimit = bound(p, ui);

//       dx = 0.1;

//       tmin = minimize1d(fi, {
//         lowerBound: tlimit[0],
//         upperBound: tlimit[1],
//         initialIncrement: dx,
//         tolerance: dx * tol1d
//       });

//       if (tmin === 0) {
//         return p;
//       }

//       err = 0;
//       for (j = 0; j < n; j++) {
//         du = tmin * ui[j];
//         err += du * du;
//         p[j] += du;
//       }

//       constrain(p);

//       if (status) status.points.push(p.slice());

//       err = Math.sqrt(err);

//       if (verbose) console.log('Iteration ' + iter + ': ' + (err / perr) + ' f(' + p + ') = ' + f(p));

//       if (err / perr < tol) return p;

//       perr = err;
//     }

//     return p;
//   }
