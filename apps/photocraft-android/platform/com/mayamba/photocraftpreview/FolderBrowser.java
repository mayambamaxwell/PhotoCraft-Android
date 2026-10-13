package com.mayamba.photocraftpreview;
import android.app.*;
import android.content.*;
import android.database.Cursor;
import android.graphics.*;
import android.graphics.drawable.Drawable;
import android.net.Uri;
import android.os.Bundle;
import android.provider.DocumentsContract;
import android.util.LruCache;
import android.view.*;
import android.widget.*;
import android.text.*;
import java.io.*;
import java.util.*;
import java.util.concurrent.*;

/** Native Android folder browser. Provider reads and bounded previews run away from the UI thread. */
final class FolderBrowser extends Dialog {
 interface Actions {void choose(Uri uri,String name);void folder();void system(Uri current);void cancel();}
 private final Activity owner;private final Uri tree;private final boolean save;private final String suggested;private final Actions actions;
 private final ArrayList<BrowserLogic.Entry> path=new ArrayList<>();private List<BrowserLogic.Entry> entries=new ArrayList<>(),shown=new ArrayList<>();
 private final ExecutorService listing=Executors.newSingleThreadExecutor();
 private final ThreadPoolExecutor thumbnails=new ThreadPoolExecutor(2,2,0,TimeUnit.SECONDS,new ArrayBlockingQueue<Runnable>(32));
 private final Set<String> pending=new HashSet<>(),failed=new HashSet<>();
 private final LruCache<String,Bitmap> cache=new LruCache<String,Bitmap>(8<<20){protected int sizeOf(String key,Bitmap b){return b.getAllocationByteCount();}};
 private LinearLayout crumbs;private TextView status;private EditText search,name;private CheckBox imageOnly,newest;private GridView grid;private Button up,saveButton;
 private final android.os.CancellationSignal previewCancel=new android.os.CancellationSignal();private volatile android.os.CancellationSignal querySignal;
 private volatile boolean alive=true;private volatile int generation;private boolean handled,loading;private Adapter adapter;
 FolderBrowser(Activity c,Uri tree,boolean save,String suggested,Actions actions){super(c,android.R.style.Theme_Material_Dialog_NoActionBar);this.owner=c;this.tree=tree;this.save=save;this.suggested=suggested;this.actions=actions;String id=DocumentsContract.getTreeDocumentId(tree);String title=id.substring(id.lastIndexOf(':')+1);path.add(new BrowserLogic.Entry(id,title.isEmpty()?"Selected folder":title,DocumentsContract.Document.MIME_TYPE_DIR,0,0,true));}
 private int dp(int n){return Math.round(n*getContext().getResources().getDisplayMetrics().density);}
 private LinearLayout row(){LinearLayout r=new LinearLayout(getContext());r.setGravity(Gravity.CENTER_VERTICAL);return r;}
 private Button button(LinearLayout row,String text,Runnable action){Button b=new Button(getContext());b.setText(text);b.setMinHeight(dp(48));b.setOnClickListener(v->action.run());row.addView(b);return b;}
 @Override protected void onCreate(Bundle state){super.onCreate(state);LinearLayout body=new LinearLayout(getContext());body.setOrientation(LinearLayout.VERTICAL);body.setPadding(dp(12),dp(10),dp(12),dp(10));
  TextView title=new TextView(getContext());title.setText(save?"Save in folder":"Open from folder");title.setTextSize(21);body.addView(title);
  LinearLayout controls=row();up=button(controls,"Up",()->{if(path.size()>1){path.remove(path.size()-1);load();}});button(controls,"Choose folder",()->finish(()->actions.folder()));button(controls,"System Files",()->finish(()->actions.system(document(path.get(path.size()-1).id))));button(controls,"Refresh",()->load());body.addView(controls);
  HorizontalScrollView scroll=new HorizontalScrollView(getContext());crumbs=row();scroll.addView(crumbs);body.addView(scroll);
  search=new EditText(getContext());search.setSingleLine(true);search.setHint("Search this folder");body.addView(search);search.addTextChangedListener(new TextWatcher(){public void beforeTextChanged(CharSequence s,int st,int count,int after){}public void onTextChanged(CharSequence s,int st,int before,int count){filter();}public void afterTextChanged(Editable e){}});
  LinearLayout filters=row();imageOnly=new CheckBox(getContext());imageOnly.setText("Images / projects");imageOnly.setChecked(!save);filters.addView(imageOnly);newest=new CheckBox(getContext());newest.setText("Newest first");filters.addView(newest);imageOnly.setOnCheckedChangeListener((b,on)->filter());newest.setOnCheckedChangeListener((b,on)->filter());body.addView(filters);
  status=new TextView(getContext());status.setPadding(0,dp(4),0,dp(4));body.addView(status);
  grid=new GridView(getContext());grid.setNumColumns(GridView.AUTO_FIT);grid.setColumnWidth(dp(164));grid.setHorizontalSpacing(dp(8));grid.setVerticalSpacing(dp(8));grid.setStretchMode(GridView.STRETCH_COLUMN_WIDTH);adapter=new Adapter();grid.setAdapter(adapter);grid.setOnItemClickListener((parent,v,index,id)->{if(loading||index<0||index>=shown.size())return;BrowserLogic.Entry e=shown.get(index);if(e.folder){if(path.size()>=BrowserLogic.MAX_DEPTH){status.setText("Folder nesting limit reached. Choose a closer folder.");return;}path.add(e);load();}else if(save){name.setText(e.name);}else finish(()->actions.choose(document(e.id),e.name));});body.addView(grid,new LinearLayout.LayoutParams(-1,0,1));
  if(save){name=new EditText(getContext());name.setSingleLine(true);name.setHint("File name");name.setText(suggested);body.addView(name);}
  LinearLayout footer=row();if(save)saveButton=button(footer,"Save here",()->saveHere());button(footer,"Cancel",()->{cancel();});body.addView(footer);setContentView(body);setOnCancelListener(d->actions.cancel());setOnDismissListener(d->{alive=false;generation++;if(querySignal!=null)querySignal.cancel();previewCancel.cancel();listing.shutdownNow();thumbnails.shutdownNow();cache.evictAll();});load();
 }
 @Override public void show(){super.show();Window w=getWindow();if(w!=null)w.setLayout((int)(getContext().getResources().getDisplayMetrics().widthPixels*.92),(int)(getContext().getResources().getDisplayMetrics().heightPixels*.9));}
 private void finish(Runnable next){if(handled)return;handled=true;dismiss();next.run();}
 private void ui(Runnable r){owner.runOnUiThread(()->{if(alive&&isShowing())r.run();});}
 private Uri document(String id){return DocumentsContract.buildDocumentUriUsingTree(tree,id);}
 private void filter(){if(!alive||adapter==null||imageOnly==null)return;shown=BrowserLogic.filter(entries,search.getText().toString(),imageOnly.isChecked(),newest.isChecked());adapter.notifyDataSetChanged();if(!loading)status.setText((entries.size()>=BrowserLogic.MAX_ENTRIES?"First 2048 items · ":"")+shown.size()+" items"+(shown.isEmpty()?" · Folder is empty or no matches":""));}
 private void load(){final int version=++generation;if(querySignal!=null)querySignal.cancel();final android.os.CancellationSignal signal=new android.os.CancellationSignal();querySignal=signal;loading=true;pending.clear();failed.clear();entries=new ArrayList<>();filter();status.setText("Loading folder…");up.setEnabled(path.size()>1);if(saveButton!=null)saveButton.setEnabled(false);crumbs.removeAllViews();for(int i=0;i<path.size();i++){final int level=i;button(crumbs,path.get(i).name,()->{while(path.size()>level+1)path.remove(path.size()-1);load();});}
  final Uri children=DocumentsContract.buildChildDocumentsUriUsingTree(tree,path.get(path.size()-1).id);listing.execute(()->{List<BrowserLogic.Entry> found=new ArrayList<>();try(Cursor c=getContext().getContentResolver().query(children,new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME,DocumentsContract.Document.COLUMN_MIME_TYPE,DocumentsContract.Document.COLUMN_SIZE,DocumentsContract.Document.COLUMN_LAST_MODIFIED},null,null,null,signal)){if(c==null)throw new IOException("Folder provider returned no listing");while(found.size()<BrowserLogic.MAX_ENTRIES&&c.moveToNext()){if(!alive||generation!=version)return;String mime=c.getString(2);if(c.isNull(0))continue;found.add(new BrowserLogic.Entry(c.getString(0),c.getString(1),mime,c.isNull(3)?0:c.getLong(3),c.isNull(4)?0:c.getLong(4),DocumentsContract.Document.MIME_TYPE_DIR.equals(mime)));}ui(()->{if(version!=generation)return;entries=found;loading=false;if(saveButton!=null)saveButton.setEnabled(true);filter();});}catch(Exception e){ui(()->{if(version!=generation)return;loading=false;status.setText("Cannot read folder: "+e.getMessage()+". Choose folder again or use System Files.");});}});
 }
 private void saveHere(){if(loading)return;final String fileName;try{fileName=BrowserLogic.validName(name.getText().toString());}catch(IllegalArgumentException e){status.setText(e.getMessage());return;}for(BrowserLogic.Entry e:entries)if(e.name.equals(fileName)){if(e.folder){status.setText("A folder already has this name. Choose another name.");return;}new AlertDialog.Builder(getContext()).setTitle("Replace file?").setMessage(fileName+" already exists.").setNegativeButton("Cancel",null).setPositiveButton("Replace",(d,which)->finish(()->actions.choose(document(e.id),fileName))).show();return;}
  saveButton.setEnabled(false);status.setText("Creating destination…");final Uri parent=document(path.get(path.size()-1).id);listing.execute(()->{try{Uri uri=DocumentsContract.createDocument(getContext().getContentResolver(),parent,PhotoCraftActivity.mime(fileName),fileName);if(uri==null)throw new IOException("This folder cannot create files");ui(()->finish(()->actions.choose(uri,fileName)));}catch(Exception e){ui(()->{saveButton.setEnabled(true);status.setText("Cannot save here: "+e.getMessage()+". Try System Files or another folder.");});}});
 }
 private final class Adapter extends BaseAdapter{
  public int getCount(){return shown.size();}public Object getItem(int i){return shown.get(i);}public long getItemId(int i){return i;}
  public View getView(int i,View recycled,ViewGroup parent){LinearLayout cell;if(recycled instanceof LinearLayout)cell=(LinearLayout)recycled;else{cell=new LinearLayout(getContext());cell.setOrientation(LinearLayout.VERTICAL);cell.setPadding(dp(8),dp(8),dp(8),dp(8));cell.setBackgroundColor(0xff303030);ImageView pic=new ImageView(getContext());pic.setScaleType(ImageView.ScaleType.FIT_CENTER);cell.addView(pic,new LinearLayout.LayoutParams(-1,dp(96)));TextView label=new TextView(getContext());label.setTextColor(0xffeeeeee);label.setMaxLines(2);label.setMinHeight(dp(44));cell.addView(label);TextView detail=new TextView(getContext());detail.setTextColor(0xffbbbbbb);detail.setTextSize(11);cell.addView(detail);}
   BrowserLogic.Entry e=shown.get(i);ImageView image=(ImageView)cell.getChildAt(0);image.setTag(e.id);if(e.folder)image.setImageDrawable(new FolderIcon());else image.setImageResource(android.R.drawable.ic_menu_gallery);((TextView)cell.getChildAt(1)).setText(e.name);((TextView)cell.getChildAt(2)).setText(e.folder?"Folder":e.size>0?String.format(Locale.ROOT,"%.1f MB",e.size/1048576.):e.mime);if(!e.folder&&e.mime.startsWith("image/")){Bitmap cached=cache.get(e.id);if(cached!=null)image.setImageBitmap(cached);else thumbnail(e,image);}return cell;
  }
 }
 private static final class Limited extends FilterInputStream{
  private long remaining=16L<<20;Limited(InputStream in){super(in);}
  public int read()throws IOException{if(remaining<=0)throw new IOException("Preview byte limit");int n=super.read();if(n>=0)remaining--;return n;}
  public int read(byte[] b,int offset,int count)throws IOException{if(count==0)return 0;if(remaining<=0)throw new IOException("Preview byte limit");int n=super.read(b,offset,(int)Math.min(remaining,count));if(n>0)remaining-=n;return n;}
  public long skip(long count)throws IOException{long n=super.skip(Math.min(remaining,count));remaining-=n;return n;}
 }
 private static final class FolderIcon extends Drawable{
  private final Paint paint=new Paint(Paint.ANTI_ALIAS_FLAG);
  public void draw(Canvas c){Rect b=getBounds();int saved=c.save();c.translate(b.left,b.top);c.scale(b.width()/96f,b.height()/96f);paint.setColor(0xffb9872e);c.drawRoundRect(new RectF(8,18,47,48),5,5,paint);paint.setColor(0xffe3b653);c.drawRoundRect(new RectF(5,32,91,79),6,6,paint);c.restoreToCount(saved);}
  public void setAlpha(int alpha){paint.setAlpha(alpha);}public void setColorFilter(ColorFilter f){paint.setColorFilter(f);}public int getOpacity(){return PixelFormat.TRANSLUCENT;}public int getIntrinsicWidth(){return 96;}public int getIntrinsicHeight(){return 96;}
 }
 private void thumbnail(BrowserLogic.Entry entry,ImageView view){if(failed.contains(entry.id)||!pending.add(entry.id))return;final int version=generation;try{thumbnails.execute(()->{if(!alive||generation!=version)return;Bitmap bitmap=null;try{Uri uri=document(entry.id);try{bitmap=getContext().getContentResolver().loadThumbnail(uri,new android.util.Size(192,192),previewCancel);}catch(Exception ignored){}if(!alive||generation!=version)return;if(bitmap==null){BitmapFactory.Options bounds=new BitmapFactory.Options();bounds.inJustDecodeBounds=true;try(InputStream in=getContext().getContentResolver().openInputStream(uri)){BitmapFactory.decodeStream(new Limited(in),null,bounds);}if(bounds.outWidth>0&&bounds.outHeight>0){BitmapFactory.Options options=new BitmapFactory.Options();options.inSampleSize=BrowserLogic.sampleSize(bounds.outWidth,bounds.outHeight);options.inPreferredConfig=Bitmap.Config.RGB_565;try(InputStream in=getContext().getContentResolver().openInputStream(uri)){bitmap=BitmapFactory.decodeStream(new Limited(in),null,options);}}}}catch(Exception|OutOfMemoryError ignored){}final Bitmap ready=bitmap;ui(()->{if(version!=generation)return;pending.remove(entry.id);if(ready==null)failed.add(entry.id);if(ready!=null&&ready.getAllocationByteCount()<1<<20){cache.put(entry.id,ready);if(entry.id.equals(view.getTag()))view.setImageBitmap(ready);adapter.notifyDataSetChanged();}});});}catch(RejectedExecutionException e){pending.remove(entry.id);}}
}
