"""Reference-side design of the original seven-rate V10/V11 diagnostic filter."""
import json
from pathlib import Path
from scipy import signal
import sys
result={}
for rate in (32000,44100,48000,88200,96000,176400,192000):
    sos=signal.butter(3,(150,290),fs=rate,btype="bandpass",output="sos")
    result[f'{rate}/196']=dict(sos=sos.tolist(),initial=signal.sosfilt_zi(sos).tolist())
Path(sys.argv[1]).write_text(json.dumps(result,allow_nan=False,separators=(',',':'))+'\n')
