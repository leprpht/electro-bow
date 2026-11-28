// Offline binary64 sample transaction. A failed Epoch is discarded by its lane owner.
#include <algorithm>
#include <array>
#include <cmath>
#include <complex>
#include <cstddef>
#include <cstring>
#include <limits>
#include <new>

namespace {
constexpr double tau = 6.283185307179586476925286766559;
using Vector = std::array<double, 6>;
using Quad = std::array<double, 3>;
enum class Fault {
    phase = 1,
    coefficient,
    bank,
    trial,
    moment,
    normalize,
    gram,
    residual,
    wet,
    conversion
};
double checked(double x, Fault site) {
    if (!std::isfinite(x))
        throw site;
    return x;
}
struct Stats {
    double gram[21]{};
    Vector u[13]{}; // q,d,y interleaved by path, then M5
    Quad moments[4]{};
    double y2[4]{}, m2{}, pair[3]{}, pair_y2{}, pre2{}, qy{}, dy{};
};
static_assert(sizeof(Stats) == 984);
constexpr int ix(int i, int j) {
    return i * (i + 1) / 2 + j;
}

struct Epoch {
    Stats blocks[2]{};
    std::complex<double> z[4][12]{}, zz[4][12]{}, poles[12]{};
    double taper[12]{}, last_pitch{}, radius{}, phase{}, scale{}, target{}, selected{};
    int current{}, count{}, length{}, retired_n{}, trial{-1}, ready{}, best{-1};
    bool observed{true}, valid{}, bridge{}, available{}, live{}, veto{true}, pending{};
    double g(int i, int j) const {
        return blocks[1 - current].gram[ix(std::max(i, j), std::min(i, j))];
    }
    double positive(double x, double mag) const {
        checked(x, Fault::residual);
        double tol = checked((64 * retired_n * std::numeric_limits<double>::epsilon()) * mag,
                             Fault::residual);
        if (x < -tol)
            throw Fault::residual;
        return std::max(0., x);
    }
    double energy(const Vector& a, int lo, int hi) const {
        double s = 0, mag = 0;
        for (int i = lo; i < hi; ++i)
            for (int j = lo; j < hi; ++j) {
                double term = (a[i] * g(i, j)) * a[j];
                s += term;
                mag += std::abs(term);
            }
        return positive(s, mag);
    }
    double power(const double* m, double w) const {
        double cross = 2 * w * m[1], dd = (w * w) * m[2];
        return positive((m[0] + cross) + dd, std::abs(m[0]) + std::abs(cross) + std::abs(dd));
    }
    bool passes(Vector a, double p, double neutral, bool recovery) const {
        const auto& s = blocks[1 - current];
        double fit = energy(a, 0, 6);
        double rest = positive(p - fit, std::abs(p) + fit);
        double h1 = std::sqrt(energy(a, 1, 3)), h2 = std::sqrt(energy(a, 3, 5));
        double competitor = std::max({h2, std::sqrt(2.) * std::abs(a[5]), std::sqrt(rest)});
        double factor = std::pow(10., (recovery ? 2.85 : 3.) / 10);
        double margin = std::pow(10., (recovery ? .30 : .05) / 20);
        return s.m2 > 1e-24 && neutral > 1e-24 && h1 >= margin * competitor + 1e-12 &&
               p >= s.m2 / factor && p <= checked(s.m2 * factor, Fault::trial) &&
               p >= neutral / factor && p <= checked(neutral * factor, Fault::trial);
    }
    void retire(bool eligible, int probe) {
        current = 1 - current;
        auto& s = blocks[1 - current];
        retired_n = length;
        double reciprocal = probe == 6 ? std::numeric_limits<double>::max() : 1. / retired_n;
        auto norm = [&](double& x) { x = checked(x * reciprocal, Fault::normalize); };
        for (auto& v : s.gram)
            norm(v);
        for (auto& row : s.u)
            for (auto& v : row)
                norm(v);
        for (auto& row : s.moments)
            for (auto& v : row)
                norm(v);
        for (auto& v : s.y2)
            norm(v);
        norm(s.m2);
        for (auto& v : s.pair)
            norm(v);
        norm(s.pair_y2);
        norm(s.pre2);
        norm(s.qy);
        norm(s.dy);
        if (probe == 7)
            s.gram[0] = std::numeric_limits<double>::quiet_NaN();
        valid = true;
        for (int i = 0; i < 6; ++i) {
            double r = 0;
            for (int j = 0; j < 6; ++j)
                if (i != j)
                    r += std::abs(g(i, j));
            valid &=
                checked(g(i, i) - r, Fault::gram) >= .25 && checked(g(i, i) + r, Fault::gram) <= 2.;
        }
        live = bridge = false;
        if (valid) {
            double l[21]{};
            for (int i = 0; i < 6; ++i)
                for (int j = 0; j <= i; ++j) {
                    double v = g(i, j);
                    for (int k = 0; k < j; ++k)
                        v -= l[ix(i, k)] * l[ix(j, k)];
                    l[ix(i, j)] = checked(i == j ? std::sqrt(v) : v / l[ix(j, j)], Fault::gram);
                }
            for (auto& a : s.u) {
                for (int i = 0; i < 6; ++i) {
                    for (int j = 0; j < i; ++j)
                        a[i] -= l[ix(i, j)] * a[j];
                    a[i] = checked(a[i] / l[ix(i, i)], Fault::gram);
                }
                for (int i = 5; i >= 0; --i) {
                    for (int j = i + 1; j < 6; ++j)
                        a[i] -= l[ix(j, i)] * a[j];
                    a[i] = checked(a[i] / l[ix(i, i)], Fault::gram);
                }
            }
            bridge = passes(s.u[12], s.m2, s.m2, false);
            if (probe == 8)
                s.y2[3] = -1;
            live = true;
            for (int j = 0; j < 4; ++j)
                live &= passes(s.u[3 * j + 2], s.y2[j], s.moments[j][0], false);
            live &= s.pair_y2 <= std::pow(10., -2.8) * s.y2[0];
        }
        available = valid && bridge && eligible && observed;
        phase = checked(std::fmod(phase, tau), Fault::phase);
        observed = true;
        count = 0;
        trial = 0;
        best = -1;
        blocks[current] = {};
    }
    bool candidate(double w, int probe) {
        auto& s = blocks[1 - current];
        if (probe == 4)
            s.moments[0][1] = std::numeric_limits<double>::max();
        if (probe == 14)
            s.u[0][0] = std::numeric_limits<double>::quiet_NaN();
        bool ok = valid;
        if (valid) {
            for (int j = 0; j < 3; ++j) {
                Vector a{};
                for (int k = 0; k < 6; ++k)
                    a[k] = checked(s.u[3 * j][k] + w * s.u[3 * j + 1][k], Fault::trial);
                double p = power(s.moments[j].data(), w);
                ok &= passes(a, p, s.moments[j][0], true);
            }
            ok &= power(s.pair, w) <= std::pow(10., -2.85) * power(s.moments[0].data(), w);
        }
        return ok;
    }
    void tick(double fs, const double* in, double* out, int probe) {
        const double f = in[0], m = in[1], *q = in + 2, *c = in + 6;
        if (!in[19]) {
            available = false;
            pending = false;
            target = 0.;
            selected = 0.;
            ready = 0;
            veto = true;
        }
        if (pending) {
            target = selected;
            pending = false;
        }
        checked(f, Fault::phase);
        if (count == 0) {
            if (f < 65 || f > 1320)
                throw Fault::phase;
            length = 2 * int(std::floor(4 * fs / f + .5));
        }
        if (f != last_pitch) {
            radius = std::exp(-tau * .22 * f / fs);
            double fa = fs / std::floor(fs / 16000);
            for (int k = 0; k < 12; ++k) {
                poles[k] = std::polar(radius, tau * (k + 1) * f / fs);
                taper[k] = std::clamp((.28 * fa - (k + 1) * f) / (.03 * fa), 0., 1.);
            }
            last_pitch = f;
        }
        double d[4]{};
        if (probe == 3)
            zz[3][0] = {INFINITY, 0};
        if (probe == 13) {
            z[3][0] = {std::numeric_limits<double>::max(), std::numeric_limits<double>::max()};
            zz[3][0] = z[3][0];
        }
        for (int j = 0; j < 4; ++j)
            for (int k = 0; k < 12; ++k) {
                z[j][k] = poles[k] * z[j][k] + (1 - radius) * q[j];
                zz[j][k] = poles[k] * zz[j][k] + (1 - radius) * z[j][k];
                for (auto v : {z[j][k], zz[j][k]}) {
                    checked(v.real(), Fault::bank);
                    checked(v.imag(), Fault::bank);
                }
                double coeff = checked(probe == 2 && k == 0 ? NAN : c[k], Fault::coefficient);
                d[j] = checked(d[j] + coeff * taper[k] * 2 * zz[j][k].real(), Fault::bank);
            }
        if (probe == 1)
            phase = NAN;
        phase = checked(phase + (probe == 11 ? INFINITY : tau * f / fs), Fault::phase);
        Vector b = {1,
                    std::cos(phase),
                    std::sin(phase),
                    std::cos(2 * phase),
                    std::sin(2 * phase),
                    count % 2 ? -1. : 1.};
        out[9] = trial;
        out[19] = -1;
        if (trial >= 0) {
            bool ok = candidate((16 - trial) / 16., probe);
            out[19] = ok;
            if (ok && best < 0)
                best = 16 - trial;
            if (++trial == 17) {
                ready = ok && available && live ? std::min(2, ready + 1) : 0;
                veto = ready < 2;
                selected = veto ? 0. : best / 16.;
                pending = true;
                trial = -1;
            }
        }
        double step = 1 / std::floor(.005 * fs + .5);
        scale += std::clamp(target - scale, -step, step);
        auto& s = blocks[current];
        auto add = [](double& x, double a, double v) { x = checked(x + a * v, Fault::moment); };
        double y[4];
        for (int j = 0; j < 4; ++j) {
            double direct = q[j], delta = d[j], gain = scale;
            if (probe == 9 && j == 3) {
                direct = delta = std::numeric_limits<double>::max();
                gain = 1.;
            }
            y[j] = checked(gain == 0 ? direct : direct + gain * delta, Fault::wet);
            out[j] = d[j];
            out[4 + j] = y[j];
        }
        for (int i = 0; i < 6; ++i) {
            for (int k = 0; k <= i; ++k)
                add(s.gram[ix(i, k)], b[i], b[k]);
            for (int j = 0; j < 4; ++j) {
                add(s.u[3 * j][i], b[i], q[j]);
                add(s.u[3 * j + 1][i], b[i], d[j]);
                add(s.u[3 * j + 2][i], b[i], y[j]);
            }
            add(s.u[12][i], b[i], m);
        }
        for (int j = 0; j < 4; ++j) {
            auto& a = s.moments[j];
            add(a[0], q[j], q[j]);
            add(a[1], q[j], d[j]);
            add(a[2], d[j], d[j]);
            add(s.y2[j], y[j], y[j]);
        }
        add(s.m2, m, probe == 5 ? std::numeric_limits<double>::infinity() : m);
        double x = q[1] - q[0], v = d[1] - d[0], diff = y[1] - y[0], pre = q[3] + target * d[3];
        add(s.pair[0], x, x);
        add(s.pair[1], x, v);
        add(s.pair[2], v, v);
        add(s.pair_y2, diff, diff);
        add(s.pre2, pre, pre);
        add(s.qy, q[3], y[3]);
        add(s.dy, d[3], y[3]);
        observed &= in[18] != 0 && in[19] != 0;
        out[10] = ++count == length;
        if (count == length)
            retire(in[19] != 0, probe);
        double terminal = probe == 10 ? std::numeric_limits<double>::max() : y[3];
        if (std::abs(terminal) > std::numeric_limits<float>::max())
            throw Fault::conversion;
        out[8] = scale;
        out[11] = live;
        out[12] = valid;
        out[13] = bridge;
        out[14] = available;
        out[15] = veto;
        out[16] = selected;
        out[17] = phase;
        out[18] = target;
    }
};
} // namespace
static_assert(sizeof(Epoch) == 3880);
static_assert(offsetof(Stats, pre2) == 960);
static_assert(offsetof(Epoch, current) == 3840);
extern "C" std::size_t epoch_size() {
    return sizeof(Epoch);
}
extern "C" void epoch_init(void* memory) {
    new (memory) Epoch{};
}
extern "C" int epoch_run(Epoch* epoch, double fs, const double* inputs, double* outputs, int n,
                         int probe, int* done) {
    for (*done = 0; *done < n; ++*done) {
        try {
            epoch->tick(fs, inputs + 20 * (*done), outputs + 20 * (*done), probe);
        } catch (Fault site) {
            return static_cast<int>(site);
        }
    }
    return 0;
}
// Diagnostic copy of the live sums or retired solved block, outside the sample path.
extern "C" void epoch_stats(Epoch* e, double* dst, int retired) {
    std::memcpy(dst, &e->blocks[retired ? 1 - e->current : e->current], sizeof(Stats));
}
