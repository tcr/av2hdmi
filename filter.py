from scipy.signal import butter, filtfilt
import numpy as np

def butter_highpass(cutoff, fs, order=5):
    nyq = 0.5 * fs
    normal_cutoff = cutoff / nyq
    b, a = butter(order, normal_cutoff, btype='low', analog=False)
    return b, a

def butter_highpass_filter(data, cutoff, fs, order=5):
    b, a = butter_highpass(cutoff, fs, order=order)
    y = filtfilt(b, a, data)
    return y

rawdata = np.loadtxt('out.csv', skiprows=0)
signal = rawdata
fs = 24000.0

cutoff = 4000
order = 1
conditioned_signal = butter_highpass_filter(signal, cutoff, fs, order)
for s in conditioned_signal:
  print(s)
