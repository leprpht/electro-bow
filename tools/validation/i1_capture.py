"""Sample-indexed HP/FIR ring and bounded four-period projection job."""

import math

import numpy as np
from scipy.signal import firwin


class Source:
    def __init__(self, fs):
        self.d = int(fs // 16000)
        self.fa = fs / self.d
        self.hp_coeff = math.exp(-2 * math.pi * 30 / fs)
        self.taps = firwin(32 * self.d + 1, 0.34 * self.fa, fs=fs, window="blackman")
        self.hp = np.empty(len(self.taps))
        self.ring = np.empty(math.ceil(4 * self.fa / 65) + math.floor(0.005 * self.fa + 0.5) + 3)
        self.reset()

    def reset(self):
        self.age = self.emitted = 0
        self.prev = self.high = 0.0

    def tick(self, x):
        self.high = self.hp_coeff * (self.high + x - self.prev)
        self.prev = x
        size = len(self.hp)
        pos = self.age % size
        self.hp[pos] = self.high
        self.age += 1
        if self.age % self.d == 0:
            n = min(self.age, size)
            # Mirror-symmetric FIR, newest first. Unwritten reset prefix is zero.
            tail = min(n, pos + 1)
            y = self.taps[:tail] @ self.hp[pos + 1 - tail : pos + 1][::-1]
            if tail < n:
                y += self.taps[tail:n] @ self.hp[size - (n - tail) :][::-1]
            self.ring[self.emitted % len(self.ring)] = y
            self.emitted += 1


class Projection:
    def __init__(self, source, m, frame):
        self.source, self.m, self.frame = source, m, frame
        self.first = source.emitted - m
        self.i = 0
        self.sum_x = self.sum_w = 0.0
        self.orders = np.arange(1, min(12, int(0.28 * source.fa / frame.hz)) + 1)
        self.xw = np.zeros(len(self.orders), complex)
        self.w = self.xw.copy()

    def advance(self, budget):
        stop = min(self.i + budget, self.m)
        for i in range(self.i, stop):
            x = self.source.ring[(self.first + i) % len(self.source.ring)]
            hann = 0.5 - 0.5 * math.cos(2 * math.pi * i / (self.m - 1))
            basis = hann * np.exp(
                (-2j * math.pi * self.frame.hz * i / self.source.fa) * self.orders
            )
            self.xw += x * basis
            self.w += basis
            self.sum_x += x
            self.sum_w += hann
        self.i = stop
        if stop < self.m:
            return None
        amp = abs((2 / self.sum_w) * (self.xw - (self.sum_x / self.m) * self.w))
        eps = max(3 * self.frame.floor, 1e-5, 0.015 * max(amp))
        weights = np.clip((amp - eps) / (3 * eps), 0, 1)
        if np.count_nonzero(weights) < 2:
            return None
        levels = 20 * np.log10(self.orders * np.maximum(amp, eps))
        out = np.zeros(12)
        out[: len(amp)] = np.clip(
            0.45 * (levels - np.average(levels, weights=weights)) * weights, -5, 5
        )
        return out
