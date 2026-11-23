"""Write a standard Deflate ZIP with stronger lossless compression for the HTML.

Usage: package-mobile-preview.py NORMAL_PREVIEW MOBILE_PREVIEW OUTPUT_ZIP
Requires zopfli (one optimization iteration). The ZIP uses ordinary method 8,
so readers need no Zopfli support. It preserves every byte of the preview.
"""
import binascii, hashlib, json, struct, sys, zipfile, zlib
from pathlib import Path
import zopfli.zlib

normal,mobile,output=map(Path,sys.argv[1:4])
layout=json.loads((normal/'layout.json').read_text())
readme=(f"Noms village — revision {layout['version']}\n\n"
        "Extract OPEN-VILLAGE-MOBILE.html and open it in Chrome. All twelve locations, "
        "the landscape and viewer are embedded.\nThis is a review preview, not the deployed About page.\n").encode()
files=[('OPEN-VILLAGE-MOBILE.html',(mobile/'OPEN-VILLAGE-MOBILE.html').read_bytes()),('READ-ME.txt',readme)]
files += [(name,(normal/name).read_bytes()) for name in ['THREE-LICENSE.txt','MESHOPT-LICENSE.txt','DROID-FONT-NOTICE.txt']]
output.parent.mkdir(parents=True,exist_ok=True)
temporary=output.with_suffix(output.suffix+'.tmp');entries=[]
with temporary.open('wb') as stream:
    for name,data in files:
        encoded=name.encode('utf-8');offset=stream.tell();crc=binascii.crc32(data)&0xffffffff
        print('COMPRESSING',name,len(data),flush=True)
        compressed=zopfli.zlib.compress(data,numiterations=1) if name.endswith('.html') else zlib.compress(data,9)
        assert compressed[0]&15==8 and not (compressed[1]&32), 'Expected Deflate without a preset dictionary'
        raw=compressed[2:-4] # ordinary Deflate stream, without the zlib header/checksum
        assert max(len(data),len(raw),offset)<0xffffffff, 'ZIP64 is not supported by this small packager'
        # Deterministic 1980-01-01 timestamp; UTF-8 names, normal Deflate method 8.
        stream.write(struct.pack('<IHHHHHIIIHH',0x04034b50,20,0x800,8,0,33,crc,len(raw),len(data),len(encoded),0))
        stream.write(encoded);stream.write(raw)
        entries.append((encoded,crc,len(raw),len(data),offset))
        print('COMPRESSED',name,len(raw),flush=True)
    start=stream.tell()
    for name,crc,packed,size,offset in entries:
        stream.write(struct.pack('<IHHHHHHIIIHHHHHII',0x02014b50,20,20,0x800,8,0,33,crc,packed,size,len(name),0,0,0,0,0,offset))
        stream.write(name)
    length=stream.tell()-start
    stream.write(struct.pack('<IHHHHIIH',0x06054b50,0,0,len(entries),len(entries),length,start,0))
with zipfile.ZipFile(temporary) as archive:
    assert archive.testzip() is None
    for name,data in files:
        assert archive.getinfo(name).compress_type==zipfile.ZIP_DEFLATED
        assert hashlib.sha256(archive.read(name)).digest()==hashlib.sha256(data).digest()
temporary.replace(output)
print('ZIP_VERIFIED',output.stat().st_size,flush=True)
