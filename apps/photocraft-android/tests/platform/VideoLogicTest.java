package com.mayamba.photocraftpreview;
import java.util.Arrays;
public final class VideoLogicTest {
 public static void main(String[] args){int n=TimelapseVideo.SIDE*TimelapseVideo.SIDE;int[] pixels=new int[n];byte[] result=new byte[n*3/2];
  int[][] samples={{0xff000000,16,128,128},{0xffffffff,235,128,128},{0xffff0000,82,90,240},{0xff0000ff,41,240,110}};
  for(int[] s:samples){Arrays.fill(pixels,s[0]);TimelapseVideo.toYuv(pixels,result);if((result[0]&255)!=s[1]||(result[n]&255)!=s[2]||(result[n+n/4]&255)!=s[3])throw new AssertionError("AVC colour conversion mismatch");if(result[n-1]!=result[0])throw new AssertionError("Incomplete frame");}
  System.out.println("4 AVC colour-plane regression cases passed");
 }
}
