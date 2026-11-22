"""Denoise Cycles review renders using Open Image Denoise. Requires pyoidn, numpy, Pillow."""
import sys
from pathlib import Path
import pyoidn
import numpy as np
from PIL import Image
root=Path(__file__).resolve().parent.parent/'previews'
for key in sys.argv[1:]:
 p=root/(key+'.png');rgb=np.asarray(Image.open(p).convert('RGB'),dtype=np.float32)/255;out=np.empty_like(rgb)
 with pyoidn.Device() as d:
  d.set_int('numThreads',2);d.commit()
  with pyoidn.Filter(d,'RT') as f:
   f.set_image('color',rgb,pyoidn.OIDN_FORMAT_FLOAT3);f.set_image('output',out,pyoidn.OIDN_FORMAT_FLOAT3);f.set_bool('hdr',False);f.set_bool('srgb',True);f.set_quality(pyoidn.OIDN_QUALITY_HIGH);f.commit();f.execute()
  err=d.get_error();assert err is None,err
 Image.fromarray(np.clip(out*255,0,255).astype(np.uint8)).save(p)
 print('DENOISED',key,flush=True)
