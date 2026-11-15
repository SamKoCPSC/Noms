#!/usr/bin/env python3
"""Pixel regressions using the production WebGL shader, including stacked glass.

python scripts/tests/test-about-glass-browser.py
Requires Playwright and Chromium; no app server or model downloads needed.
"""
import asyncio
import json
from pathlib import Path
import re
from playwright.async_api import async_playwright

ROOT = Path(__file__).resolve().parents[2]
SOURCE = (ROOT / 'src/pages/about_tour/scene.rs').read_text()
SHADERS = {name.lower(): re.search(r'const ' + name + r': &str = r#"(.*?)"#;', SOURCE, re.S)[1]
           for name in ('VERTEX', 'FRAGMENT')}

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.launch(executable_path='/usr/bin/chromium', args=[
            '--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'])
        page = await browser.new_page()
        result = await page.evaluate(r'''({vertex,fragment})=>{
 const canvas=document.createElement('canvas');canvas.width=canvas.height=32;
 const gl=canvas.getContext('webgl',{alpha:true,premultipliedAlpha:true,antialias:false});
 if(!gl)throw Error('WebGL unavailable');
 const compile=(type,source)=>{const s=gl.createShader(type);gl.shaderSource(s,source);gl.compileShader(s);
  if(!gl.getShaderParameter(s,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(s));return s;};
 const program=gl.createProgram();gl.attachShader(program,compile(gl.VERTEX_SHADER,vertex));
 gl.attachShader(program,compile(gl.FRAGMENT_SHADER,fragment));gl.linkProgram(program);
 if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(program));
 gl.useProgram(program);
 const u=name=>gl.getUniformLocation(program,name);
 const f=(name,value)=>gl.uniform1f(u(name),value);
 const v=(name,value)=>gl.uniform3fv(u(name),value);
 const identity=[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1];
 for(const name of ['u_model','u_vp','u_light_mvp'])gl.uniformMatrix4fv(u(name),false,identity);
 gl.uniformMatrix3fv(u('u_normal'),false,[1,0,0,0,1,0,0,0,1]);
 v('u_eye',[0,0,1000]);v('u_key_light',[-0.4,0.8,0.4472]);
 v('u_fill_light',[0.8,0.6,0]);v('u_rim_light',[0,0.8,-0.6]);v('u_room_up',[0,1,0]);
 f('u_diorama',1);f('u_material_mode',1);f('u_reveal',2);f('u_enter',1);
 f('u_shadow_enabled',0);f('u_textured',0);f('u_shadow',0);
 const texture=gl.createTexture();gl.bindTexture(gl.TEXTURE_2D,texture);
 gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,1,1,0,gl.RGBA,gl.UNSIGNED_BYTE,new Uint8Array([255,255,255,255]));
 gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.NEAREST);
 const buffer=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,buffer);
 gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([-1,-1,0,1,-1,0,1,1,0,-1,-1,0,1,1,0,-1,1,0]),gl.STATIC_DRAW);
 const pos=gl.getAttribLocation(program,'a_pos');gl.enableVertexAttribArray(pos);gl.vertexAttribPointer(pos,3,gl.FLOAT,false,0,0);
 const normal=gl.getAttribLocation(program,'a_normal'),color=gl.getAttribLocation(program,'a_color');
 gl.vertexAttrib2f(gl.getAttribLocation(program,'a_uv'),0,0);
 gl.enable(gl.BLEND);gl.enable(gl.DEPTH_TEST);gl.depthFunc(gl.LESS);
 const over=()=>gl.blendFuncSeparate(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA,gl.ONE,gl.ONE_MINUS_SRC_ALPHA);
 function render({background=[0,0,0,1],tint=[0.9,0.95,1],layers=1,angle=0,enter=1,occluded=false,mode='glass',density=0.2}={}){
  gl.depthMask(true);gl.clearColor(...background);gl.clearDepth(occluded?0:1);gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT);
  gl.depthMask(false);gl.vertexAttrib3f(normal,Math.sin(angle),0,Math.cos(angle));gl.vertexAttrib4f(color,...tint,1);
  gl.uniform4f(u('u_color'),1,1,1,density);f('u_enter',enter);
  for(let i=0;i<layers;i++){
   if(mode==='glass'){
    f('u_glass_pass',1);gl.blendFuncSeparate(gl.ZERO,gl.SRC_COLOR,gl.ZERO,gl.ONE);gl.drawArrays(gl.TRIANGLES,0,6);
    f('u_glass_pass',2);over();gl.drawArrays(gl.TRIANGLES,0,6);
   }else if(mode==='filter'){
    f('u_glass_pass',1);gl.blendFuncSeparate(gl.ZERO,gl.SRC_COLOR,gl.ZERO,gl.ONE);gl.drawArrays(gl.TRIANGLES,0,6);
   }else{f('u_glass_pass',0);gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);gl.drawArrays(gl.TRIANGLES,0,6);}
  }
  const pixel=new Uint8Array(4);gl.readPixels(16,16,1,1,gl.RGBA,gl.UNSIGNED_BYTE,pixel);
  if(gl.getError()!==gl.NO_ERROR)throw Error('WebGL error');return [...pixel];
 }
 return {
  oldStack:render({layers:4,mode:'matte'}),newStack:render({layers:4}),
  clear:render({background:[0.5,0.3,0.1,1],layers:0}),
  filtered:render({background:[0.5,0.3,0.1,1],mode:'filter',layers:4}),
  tinted:render({background:[0.5,0.5,0.5,1],tint:[0.05,0.25,0.75],density:0.58}),
  front:render(),edge:render({angle:1.4}),back:render({angle:Math.PI}),
  hidden:render({background:[0.5,0.3,0.1,1],enter:0}),
  occluded:render({background:[0.5,0.3,0.1,1],occluded:true}),
  transparent:render({background:[0,0,0,0]})
 };
}''', SHADERS)
        assert max(result['newStack'][:3]) < 32, result
        assert sum(result['newStack'][:3]) < sum(result['oldStack'][:3]) / 2, result
        assert all(a <= b for a, b in zip(result['filtered'], result['clear'])), result
        assert result['tinted'][0] < result['tinted'][1] < result['tinted'][2], result
        assert sum(result['edge'][:3]) > sum(result['front'][:3]), result
        assert all(abs(a-b) <= 1 for a,b in zip(result['front'],result['back'])), result
        assert result['hidden'] == result['occluded'] == result['clear'], result
        assert result['oldStack'][3] < 255 and result['newStack'][3] == 255, result
        # Source-over alpha must be F, not F squared, on the transparent page canvas.
        assert 9 <= result['transparent'][3] <= 11, result
        assert max(result['transparent'][:3]) <= result['transparent'][3], result
        await browser.close()
        print(json.dumps(result, indent=2))
        print('PASS: stacked glass, absorption, tint, Fresnel, two-sided normals, entry, depth and canvas alpha')

if __name__ == '__main__':
    asyncio.run(main())
