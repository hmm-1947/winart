package android.content;

public class ContextWrapper extends Context {
    private String packageName = "";

    public ContextWrapper() {}

    public void winrtSetPackage(String p) {
        packageName = p;
    }

    @Override
    public String getPackageName() {
        return packageName;
    }
}
