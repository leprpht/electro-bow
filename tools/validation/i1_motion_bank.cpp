// Offline differential bank for four synchronized O2 returns.
#include <algorithm>
#include <array>
#include <cmath>
#include <complex>
#include <cstdio>
#include <cstdlib>

int main(int argc, char** argv) {
    if (argc != 2)
        return 1;
    const double rate = std::atof(argv[1]);
    const double analysis_rate = rate / std::floor(rate / 16000);
    const double tau = 2 * std::acos(-1.0);
    using Complex = std::complex<double>;
    std::array<std::array<Complex, 12>, 4> z{}, w{};
    double previous_pitch = 0, radius = 0;
    std::array<Complex, 12> p{};
    std::array<double, 12> taper{};
    std::array<double, 17> frame{};
    while (std::fread(frame.data(), sizeof(double), frame.size(), stdin) == frame.size()) {
        const double f = frame[0];
        if (f != previous_pitch) {
            previous_pitch = f;
            radius = std::exp(-tau * .22 * f / rate);
            for (int k = 0; k < 12; ++k) {
                p[k] = std::polar(radius, tau * (k + 1) * f / rate);
                taper[k] =
                    std::clamp((.28 * analysis_rate - (k + 1) * f) / (.03 * analysis_rate), 0., 1.);
            }
        }
        std::array<double, 6> result{};
        for (int j = 0; j < 4; ++j) {
            result[5] = std::max(result[5], std::abs(frame[j + 1]));
            for (int k = 0; k < 12; ++k) {
                z[j][k] = p[k] * z[j][k] + (1 - radius) * frame[j + 1];
                w[j][k] = p[k] * w[j][k] + (1 - radius) * z[j][k];
                result[j] += frame[5 + k] * taper[k] * 2 * w[j][k].real();
                result[4] = std::max({result[4], std::abs(z[j][k]), std::abs(w[j][k])});
            }
        }
        if (std::fwrite(result.data(), sizeof(double), result.size(), stdout) != result.size())
            return 2;
    }
    return std::ferror(stdin) ? 3 : 0;
}
