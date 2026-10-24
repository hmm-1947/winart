package android.os;

public class Bundle {
    private String[] keys = new String[4];
    private Object[] vals = new Object[4];
    private int n;

    public Bundle() {}

    private int find(String k) {
        for (int i = 0; i < n; i++) {
            if (keys[i].equals(k)) {
                return i;
            }
        }
        return -1;
    }

    private void put(String k, Object v) {
        int i = find(k);
        if (i < 0) {
            if (n == keys.length) {
                String[] nk = new String[n * 2];
                Object[] nv = new Object[n * 2];
                for (int j = 0; j < n; j++) {
                    nk[j] = keys[j];
                    nv[j] = vals[j];
                }
                keys = nk;
                vals = nv;
            }
            i = n++;
            keys[i] = k;
        }
        vals[i] = v;
    }

    public boolean containsKey(String k) {
        return find(k) >= 0;
    }

    public int size() {
        return n;
    }

    public void putString(String k, String v) {
        put(k, v);
    }

    public String getString(String k) {
        int i = find(k);
        return i < 0 ? null : (String) vals[i];
    }

    public void putInt(String k, int v) {
        put(k, Integer.valueOf(v));
    }

    public int getInt(String k) {
        return getInt(k, 0);
    }

    public int getInt(String k, int d) {
        int i = find(k);
        return i < 0 ? d : ((Integer) vals[i]).intValue();
    }

    public void putLong(String k, long v) {
        put(k, Long.valueOf(v));
    }

    public long getLong(String k) {
        int i = find(k);
        return i < 0 ? 0L : ((Long) vals[i]).longValue();
    }

    public void putBoolean(String k, boolean v) {
        put(k, v ? Boolean.TRUE : Boolean.FALSE);
    }

    public boolean getBoolean(String k) {
        int i = find(k);
        return i >= 0 && ((Boolean) vals[i]).booleanValue();
    }
}
