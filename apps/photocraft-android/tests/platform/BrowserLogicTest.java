package com.mayamba.photocraftpreview;
import java.util.*;
public final class BrowserLogicTest {
 static void check(boolean value,String message){if(!value)throw new AssertionError(message);}
 public static void main(String[] args){
  List<BrowserLogic.Entry> files=Arrays.asList(new BrowserLogic.Entry("b","z folder","",0,0,true),new BrowserLogic.Entry("a","a folder","",0,0,true),new BrowserLogic.Entry("x","Sketch.PCRAFT","",100,10,false),new BrowserLogic.Entry("y","photo.png","image/png",30,20,false),new BrowserLogic.Entry("z","notes.txt","text/plain",0,30,false));
  List<BrowserLogic.Entry> ordered=BrowserLogic.filter(files,"",false,false);check(ordered.get(0).id.equals("a")&&ordered.get(1).folder,"Folders first / alphabetical");
  check(BrowserLogic.filter(files,"",true,false).size()==4,"Image filter retains folders and projects");check(BrowserLogic.image(new BrowserLogic.Entry("q","test.PNG","application/octet-stream",0,0,false)),"Image extension fallback");
  check(BrowserLogic.filter(files,"SKETCH",false,false).size()==1,"Case-insensitive search");
  check(BrowserLogic.filter(files,"",false,true).get(2).id.equals("z"),"Newest sorting retains folders first");
  check(BrowserLogic.validName("  Art.pcraft  ").equals("Art.pcraft"),"Save name whitespace");
  for(String n:Arrays.asList("","..","/x","a\\b","x\n.png",String.join("",Collections.nCopies(201,"x")))){boolean rejected=false;try{BrowserLogic.validName(n);}catch(IllegalArgumentException e){rejected=true;}check(rejected,"Reject unsafe save name");}
  check(BrowserLogic.sampleSize(60000,30000)==512,"Large image bounded sample");check(BrowserLogic.sampleSize(120,80)==1,"Small preview stays sharp");check(BrowserLogic.sampleSize(Integer.MAX_VALUE,1)>0,"Sampling cannot overflow");
  check(BrowserLogic.MAX_ENTRIES==2048&&BrowserLogic.MAX_DEPTH==64,"Provider limits");System.out.println("10 browser regression groups passed");
 }
}
