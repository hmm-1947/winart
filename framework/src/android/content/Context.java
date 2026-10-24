package android.content;

public abstract class Context {
    public abstract String getPackageName();

    public Context getApplicationContext() {
        return this;
    }
}
