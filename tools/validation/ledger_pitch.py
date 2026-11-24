from itertools import product
from math import ceil, floor
import numpy as np
from numpy.lib.stride_tricks import sliding_window_view
from scipy.signal import lfilter

def stationary(fs, freq, partials):
    D = fs//8000
    fa = fs/D
    H, W = round(.005*fa), ceil(fa/65)
    K, L = W+1, max(2,floor(fa/1320))
    j = np.arange(32*D+1)
    fir = np.blackman(len(j))*np.sinc(.68*(j-16*D)/D)
    hp = np.exp(-2*np.pi*30/fs)
    t = np.arange(2*fs)/fs
    x = sum(np.sin(2*np.pi*i*freq*t+.13*i*i)/i for i in range(1,partials+1))
    x = np.r_[np.zeros(round(.1*fs)),x*.35/np.max(abs(x))]
    z = lfilter(fir/fir.sum(),[1],lfilter([hp,-hp],[1,-hp],x))[D-1::D]
    errors, missing = [], 0
    for end in range(H-1,len(z),H):
        if end < len(z)-fa:
            continue
        spans = sliding_window_view(z[end-W-K+1:end+1],W)
        d = np.sum((spans[-1]-spans[::-1])**2,axis=1)
        cm = np.r_[1,d[1:]*np.arange(1,K+1)/np.cumsum(d[1:])]
        lag = next((i for i in range(L,K) if cm[i]<=cm[i-1] and
                    cm[i]<cm[i+1] and cm[i]<.15),None)
        if lag is None:
            missing += 1
            continue
        a,b,c = d[lag-1:lag+2]
        e = .5*(a-c)/(a-2*b+c) if a-2*b+c>1e-12 else 0
        f = fa/(lag+np.clip(e,-.5,.5))
        if not 65*.995 <= f <= 1320*1.005:
            missing += 1
            continue
        errors.append(abs(1200*np.log2(np.clip(f,65,1320)/freq)))
    return np.array(errors),missing

rates = (32000,44100,48000,88200,96000,176400,192000)
notes = (65.41,82.41,110,196,329.63,440,880,1318.51)
for fs,f,partials in product(rates,notes,(1,6)):
    err,missing = stationary(fs,f,partials)
    assert not missing and np.median(err)<=12 and np.percentile(err,95)<=25 and max(err)<600
print("112 clean named-grid cases passed")
for fs,f in ((48000,1230.77),(34300,1318.51)):
    err,missing = stationary(fs,f,6)
    print(fs,f,"octave errors/accepted, rejected",sum(err>600),len(err),missing,
          "absolute cents median/p95",np.percentile(err,[50,95]))
