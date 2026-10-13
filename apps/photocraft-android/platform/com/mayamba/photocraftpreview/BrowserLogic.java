package com.mayamba.photocraftpreview;
import java.util.*;
/** Pure ordering/filtering rules shared by the Android browser and host regression tests. */
public final class BrowserLogic {
 public static final int MAX_ENTRIES=2048, MAX_DEPTH=64;
 public static final class Entry {
  public final String id,name,mime;public final long size,modified;public final boolean folder;
  public Entry(String id,String name,String mime,long size,long modified,boolean folder){this.id=id;this.name=name==null?"Unnamed":name;this.mime=mime==null?"":mime;this.size=Math.max(0,size);this.modified=Math.max(0,modified);this.folder=folder;}
 }
 public static List<Entry> filter(List<Entry> entries,String query,boolean imagesOnly,boolean newest){
  String q=query.trim().toLowerCase(Locale.ROOT);List<Entry> out=new ArrayList<>();
  for(Entry e:entries)if(e.name.toLowerCase(Locale.ROOT).contains(q)&&(!imagesOnly||e.folder||image(e)))out.add(e);
  out.sort((a,b)->{if(a.folder!=b.folder)return a.folder?-1:1;if(newest){int n=Long.compare(b.modified,a.modified);if(n!=0)return n;}int n=a.name.compareToIgnoreCase(b.name);return n!=0?n:a.id.compareTo(b.id);});return out;
 }
 public static boolean image(Entry e){String n=e.name.toLowerCase(Locale.ROOT);return e.mime.startsWith("image/")||n.matches(".*\\.(png|jpe?g|webp|gif|bmp|tiff?|avif|heic|heif|exr|dng|qoi)$")||n.endsWith(".pcraft")||n.endsWith(".psd")||n.endsWith(".psb")||n.endsWith(".ora")||n.endsWith(".svg")||n.endsWith(".afphoto");}
 public static String validName(String name){String n=name.trim();if(n.isEmpty()||n.equals(".")||n.equals("..")||n.indexOf('/')>=0||n.indexOf('\\')>=0||n.length()>200)throw new IllegalArgumentException("Use a file name of 1–200 characters without slashes.");for(int i=0;i<n.length();i++)if(Character.isISOControl(n.charAt(i)))throw new IllegalArgumentException("File names cannot contain control characters.");return n;}
 public static int sampleSize(int width,int height){if(width<=0||height<=0)return 1;int sample=1;while(Math.max(width,height)/(long)sample>192&&sample<(1<<24))sample*=2;return sample;}
 private BrowserLogic(){}
}
