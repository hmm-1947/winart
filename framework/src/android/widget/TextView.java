package android.widget;

import android.content.Context;
import android.view.View;
import android.winrt.Host;
import android.winrt.Native;

public class TextView extends View {
    protected CharSequence mText = "";
    protected float mSize = 14f;
    protected boolean mSizeIsPx;
    protected int mColor = 0xFF000000;
    protected int mGravity;

    public TextView(Context context) {
        super(context);
    }

    protected int sizePx() {
        return mSizeIsPx ? (int) mSize : (int) (mSize * Host.DENSITY);
    }

    public void setText(CharSequence t) {
        mText = t == null ? "" : t;
        requestLayout();
    }

    public CharSequence getText() {
        return mText;
    }

    public void setTextSize(float sp) {
        mSize = sp;
        mSizeIsPx = false;
        requestLayout();
    }

    public void setTextSize(int unit, float v) {
        mSize = v;
        mSizeIsPx = unit == 0;
        requestLayout();
    }

    public void setTextColor(int c) {
        mColor = c;
        invalidate();
    }

    public void setGravity(int g) {
        mGravity = g;
        invalidate();
    }

    protected int minWidth() {
        return 0;
    }

    protected int minHeight() {
        return 0;
    }

    @Override
    protected void onMeasure(int ws, int hs) {
        String s = mText.toString();
        int sz = sizePx();
        int w = Native.measureText(s, sz) + mPadLeft + mPadRight;
        int h = sz + sz / 4 + mPadTop + mPadBottom;
        if (w < minWidth()) {
            w = minWidth();
        }
        if (h < minHeight()) {
            h = minHeight();
        }
        setMeasuredDimension(resolveSize(w, ws), resolveSize(h, hs));
    }

    @Override
    protected void onDrawNative(int ax, int ay) {
        String s = mText.toString();
        int sz = sizePx();
        int tw = Native.measureText(s, sz);
        int th = sz + sz / 4;
        int x = ax + mPadLeft;
        int y = ay + mPadTop;
        if ((mGravity & 1) != 0) {
            x = ax + (getWidth() - tw) / 2;
            y = ay + (getHeight() - th) / 2;
        }
        Native.drawText(s, x, y, sz, mColor);
    }
}
