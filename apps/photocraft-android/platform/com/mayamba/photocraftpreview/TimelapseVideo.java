package com.mayamba.photocraftpreview;
import android.graphics.*;
import android.media.*;
import java.io.*;
import java.nio.ByteBuffer;
import java.util.*;

/** Bounded, streaming AVC export. Only one 512px frame is decoded at a time. */
final class TimelapseVideo {
 static final int SIDE=512,FPS=24;
 static void encode(File folder,File output,int seconds)throws IOException {
  if(seconds!=10&&seconds!=30&&seconds!=60&&seconds!=120&&seconds!=300)throw new IOException("Unsupported duration");
  File[] frames=folder.listFiles((d,n)->n.endsWith(".png"));
  if(frames==null||frames.length<2)throw new IOException("Make at least two recorded edits before exporting a timelapse");
  if(frames.length>301)throw new IOException("Recording contains too many frames");
  Arrays.sort(frames,Comparator.comparing(File::getName));
  MediaCodec codec=null;MediaMuxer mux=null;boolean started=false,success=false;Bitmap bitmap=null;
  try{
   MediaFormat format=MediaFormat.createVideoFormat("video/avc",SIDE,SIDE);format.setInteger(MediaFormat.KEY_COLOR_FORMAT,MediaCodecInfo.CodecCapabilities.COLOR_FormatYUV420Flexible);format.setInteger(MediaFormat.KEY_BIT_RATE,2000000);format.setInteger(MediaFormat.KEY_FRAME_RATE,FPS);format.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL,1);
   codec=MediaCodec.createEncoderByType("video/avc");codec.configure(format,null,null,MediaCodec.CONFIGURE_FLAG_ENCODE);codec.start();
   mux=new MediaMuxer(output.getAbsolutePath(),MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4);
   int total=seconds*FPS,input=0,track=-1,last=-1;boolean eos=false;MediaCodec.BufferInfo info=new MediaCodec.BufferInfo();int[] pixels=new int[SIDE*SIDE];byte[] yuv=new byte[SIDE*SIDE*3/2];long progress=System.nanoTime();
   while(!eos){
    if(input<=total){int index=codec.dequeueInputBuffer(10000);if(index>=0){
     if(input==total){codec.queueInputBuffer(index,0,0,input*1000000L/FPS,MediaCodec.BUFFER_FLAG_END_OF_STREAM);input++;}
     else{int frame=(int)((long)input*(frames.length-1)/Math.max(1,total-1));
      if(frame!=last){if(frames[frame].length()>4L*1024*1024)throw new IOException("Recorded frame too large");BitmapFactory.Options bounds=new BitmapFactory.Options();bounds.inJustDecodeBounds=true;BitmapFactory.decodeFile(frames[frame].getPath(),bounds);if(bounds.outWidth<1||bounds.outHeight<1||bounds.outWidth>512||bounds.outHeight>512)throw new IOException("Invalid recording dimensions");
       Bitmap source=BitmapFactory.decodeFile(frames[frame].getPath());if(source==null)throw new IOException("Cannot decode recorded frame");if(bitmap==null)bitmap=Bitmap.createBitmap(SIDE,SIDE,Bitmap.Config.ARGB_8888);Canvas canvas=new Canvas(bitmap);canvas.drawColor(Color.WHITE);float scale=Math.min((float)SIDE/source.getWidth(),(float)SIDE/source.getHeight());float w=source.getWidth()*scale,h=source.getHeight()*scale;canvas.drawBitmap(source,null,new RectF((SIDE-w)/2,(SIDE-h)/2,(SIDE+w)/2,(SIDE+h)/2),new Paint(Paint.FILTER_BITMAP_FLAG));source.recycle();bitmap.getPixels(pixels,0,SIDE,0,0,SIDE,SIDE);toYuv(pixels,yuv);last=frame;}
      Image image=codec.getInputImage(index);if(image==null)throw new IOException("This device's video encoder does not support image input");try{Image.Plane[] planes=image.getPlanes();if(planes.length!=3)throw new IOException("Unsupported video colour format");int offset=0;for(int p=0;p<3;p++){int side=p==0?SIDE:SIDE/2;ByteBuffer b=planes[p].getBuffer();int base=b.position(),row=planes[p].getRowStride(),pixel=planes[p].getPixelStride();for(int y=0;y<side;y++)for(int x=0;x<side;x++)b.put(base+y*row+x*pixel,yuv[offset++]);}}finally{image.close();}
      codec.queueInputBuffer(index,0,yuv.length,input*1000000L/FPS,0);input++;
     }progress=System.nanoTime();}}
    int out=codec.dequeueOutputBuffer(info,10000);
    if(out==MediaCodec.INFO_OUTPUT_FORMAT_CHANGED){if(started)throw new IOException("Encoder changed format twice");track=mux.addTrack(codec.getOutputFormat());mux.start();started=true;}
    else if(out>=0){ByteBuffer bytes=codec.getOutputBuffer(out);if(info.size>0&&(info.flags&MediaCodec.BUFFER_FLAG_CODEC_CONFIG)==0){if(!started||bytes==null)throw new IOException("Video output unavailable");bytes.position(info.offset);bytes.limit(info.offset+info.size);mux.writeSampleData(track,bytes,info);}eos=(info.flags&MediaCodec.BUFFER_FLAG_END_OF_STREAM)!=0;codec.releaseOutputBuffer(out,false);progress=System.nanoTime();}
    if(System.nanoTime()-progress>30000000000L)throw new IOException("Video encoder stopped responding");
   }
   mux.stop();started=false;success=true;
  }catch(RuntimeException e){throw new IOException("Video encoder unavailable: "+e.getMessage(),e);}finally{if(bitmap!=null)bitmap.recycle();if(codec!=null){try{codec.stop();}catch(Exception ignored){}codec.release();}if(mux!=null){if(started)try{mux.stop();}catch(Exception ignored){}mux.release();}if(!success)output.delete();}
 }
 private static int clamp(int n){return Math.max(0,Math.min(255,n));}
 static void toYuv(int[] pixels,byte[] out){int u=SIDE*SIDE,v=u+u/4;for(int y=0;y<SIDE;y++)for(int x=0;x<SIDE;x++){int rgb=pixels[y*SIDE+x],r=(rgb>>16)&255,g=(rgb>>8)&255,b=rgb&255;out[y*SIDE+x]=(byte)clamp(((66*r+129*g+25*b+128)>>8)+16);if((y&1)==0&&(x&1)==0){out[u++]=(byte)clamp(((-38*r-74*g+112*b+128)>>8)+128);out[v++]=(byte)clamp(((112*r-94*g-18*b+128)>>8)+128);}}}
}
