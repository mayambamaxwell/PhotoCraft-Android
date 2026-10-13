package com.mayamba.photocraftpreview;
import android.content.*;
import android.database.*;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;
import java.io.*;
public final class ExportProvider extends ContentProvider {
 public boolean onCreate(){return true;}
 private File file(Uri uri)throws FileNotFoundException{if(!"PhotoCraft-share.png".equals(uri.getLastPathSegment()))throw new FileNotFoundException("Unknown export");File root=getContext().getExternalFilesDir(null);if(root==null)root=getContext().getFilesDir();return new File(root,"exports/PhotoCraft-share.png");}
 public ParcelFileDescriptor openFile(Uri uri,String mode)throws FileNotFoundException{if(!"r".equals(mode))throw new FileNotFoundException("Read-only export");return ParcelFileDescriptor.open(file(uri),ParcelFileDescriptor.MODE_READ_ONLY);}
 public String getType(Uri uri){return "image/png";}
 public Cursor query(Uri uri,String[] projection,String selection,String[] args,String order){String[] columns=projection!=null?projection:new String[]{OpenableColumns.DISPLAY_NAME,OpenableColumns.SIZE};MatrixCursor c=new MatrixCursor(columns);try{File f=file(uri);Object[] values=new Object[columns.length];for(int i=0;i<columns.length;i++)values[i]=OpenableColumns.SIZE.equals(columns[i])?f.length():OpenableColumns.DISPLAY_NAME.equals(columns[i])?f.getName():null;c.addRow(values);}catch(FileNotFoundException ignored){}return c;}
 public Uri insert(Uri u,ContentValues v){throw new UnsupportedOperationException("Read-only export");}public int delete(Uri u,String s,String[] a){return 0;}public int update(Uri u,ContentValues v,String s,String[] a){return 0;}
}
