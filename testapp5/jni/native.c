#include <jni.h>
#include <android/log.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <math.h>

#define TAG "nativetest"
#define FN(name) Java_com_winrt_nativetest_NativeLib_##name

static __thread int tls_counter = 5;
static int init_value;
static JavaVM *g_vm;
static jobject g_kept;

__attribute__((constructor)) static void on_init(void) {
    init_value = 42;
}

static jint reg_impl(JNIEnv *env, jclass c, jint x) {
    return x * 3;
}

JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM *vm, void *reserved) {
    JNIEnv *env;
    g_vm = vm;
    if ((*vm)->GetEnv(vm, (void **)&env, JNI_VERSION_1_6) != JNI_OK) {
        return -1;
    }
    jclass c = (*env)->FindClass(env, "com/winrt/nativetest/NativeLib");
    if (c == NULL) {
        return -1;
    }
    JNINativeMethod m[] = {{"registered", "(I)I", (void *)reg_impl}};
    if ((*env)->RegisterNatives(env, c, m, 1) != 0) {
        return -1;
    }
    __android_log_print(ANDROID_LOG_INFO, TAG, "JNI_OnLoad ok init=%d", init_value);
    return JNI_VERSION_1_6;
}

JNIEXPORT jint JNICALL FN(add)(JNIEnv *env, jclass c, jint a, jint b) {
    return a + b;
}

JNIEXPORT jlong JNICALL FN(mulLong)(JNIEnv *env, jclass c, jlong a, jlong b) {
    return a * b;
}

JNIEXPORT jdouble JNICALL FN(hyp)(JNIEnv *env, jclass c, jdouble a, jdouble b) {
    return sqrt(a * a + b * b);
}

JNIEXPORT jfloat JNICALL FN(scaleF)(JNIEnv *env, jclass c, jfloat a, jfloat k) {
    return a * k;
}

JNIEXPORT jboolean JNICALL FN(isEven)(JNIEnv *env, jclass c, jint n) {
    return (n & 1) == 0;
}

JNIEXPORT jbyte JNICALL FN(negByte)(JNIEnv *env, jclass c, jbyte b) {
    return (jbyte)-b;
}

JNIEXPORT jchar JNICALL FN(nextChar)(JNIEnv *env, jclass c, jchar ch) {
    return (jchar)(ch + 1);
}

JNIEXPORT jshort JNICALL FN(shortSum)(JNIEnv *env, jclass c, jshort a, jshort b) {
    return (jshort)(a + b);
}

JNIEXPORT jint JNICALL FN(many)(JNIEnv *env, jclass c, jint a, jint b, jint d, jint e, jint f, jint g, jint h, jint i) {
    return a + 2 * b + 3 * d + 4 * e + 5 * f + 6 * g + 7 * h + 8 * i;
}

JNIEXPORT jdouble JNICALL FN(manyD)(JNIEnv *env, jclass c, jdouble a, jdouble b, jdouble d, jdouble e, jdouble f, jdouble g, jdouble h, jdouble i, jdouble j, jdouble k) {
    return a + 2 * b + 3 * d + 4 * e + 5 * f + 6 * g + 7 * h + 8 * i + 9 * j + 10 * k;
}

JNIEXPORT jdouble JNICALL FN(mixed)(JNIEnv *env, jclass c, jint a, jlong b, jdouble d, jfloat f, jbyte g, jchar h) {
    return (double)a + (double)b + d + (double)f + (double)g + (double)h;
}

JNIEXPORT jstring JNICALL FN(greet)(JNIEnv *env, jclass c, jstring name, jint n) {
    const char *s = (*env)->GetStringUTFChars(env, name, NULL);
    char buf[128];
    snprintf(buf, sizeof buf, "Hello, %s! n=%d hex=%x", s, n, n);
    __android_log_print(ANDROID_LOG_INFO, TAG, "greet %s %d %.2f", s, n, 3.14159);
    (*env)->ReleaseStringUTFChars(env, name, s);
    return (*env)->NewStringUTF(env, buf);
}

JNIEXPORT jintArray JNICALL FN(strInfo)(JNIEnv *env, jclass c, jstring s) {
    jint info[3];
    info[0] = (*env)->GetStringLength(env, s);
    info[1] = (*env)->GetStringUTFLength(env, s);
    const jchar *ch = (*env)->GetStringChars(env, s, NULL);
    info[2] = info[0] > 0 ? ch[0] : -1;
    (*env)->ReleaseStringChars(env, s, ch);
    jintArray r = (*env)->NewIntArray(env, 3);
    (*env)->SetIntArrayRegion(env, r, 0, 3, info);
    return r;
}

JNIEXPORT jstring JNICALL FN(regions)(JNIEnv *env, jclass c, jstring s) {
    jchar buf[8];
    char ub[32];
    (*env)->GetStringRegion(env, s, 1, 3, buf);
    (*env)->GetStringUTFRegion(env, s, 1, 3, ub);
    jstring a = (*env)->NewString(env, buf, 3);
    jstring b = (*env)->NewStringUTF(env, ub);
    if (!(*env)->IsSameObject(env, a, b)) {
        jclass sc = (*env)->FindClass(env, "java/lang/String");
        jmethodID eq = (*env)->GetMethodID(env, sc, "equals", "(Ljava/lang/Object;)Z");
        if (!(*env)->CallBooleanMethod(env, a, eq, b)) {
            return NULL;
        }
    }
    return a;
}

JNIEXPORT jstring JNICALL FN(fromChars)(JNIEnv *env, jclass c, jcharArray a) {
    jsize n = (*env)->GetArrayLength(env, a);
    jchar *p = (*env)->GetCharArrayElements(env, a, NULL);
    jstring s = (*env)->NewString(env, p, n);
    (*env)->ReleaseCharArrayElements(env, a, p, JNI_ABORT);
    return s;
}

JNIEXPORT jstring JNICALL FN(viaCtor)(JNIEnv *env, jclass c, jcharArray a) {
    jclass sc = (*env)->FindClass(env, "java/lang/String");
    jmethodID ctor = (*env)->GetMethodID(env, sc, "<init>", "([C)V");
    return (jstring)(*env)->NewObject(env, sc, ctor, a);
}

JNIEXPORT jint JNICALL FN(sumInts)(JNIEnv *env, jclass c, jintArray a) {
    jsize n = (*env)->GetArrayLength(env, a);
    jint *p = (*env)->GetIntArrayElements(env, a, NULL);
    jint s = 0;
    for (jsize i = 0; i < n; i++) {
        s += p[i];
    }
    (*env)->ReleaseIntArrayElements(env, a, p, JNI_ABORT);
    return s;
}

JNIEXPORT void JNICALL FN(fillBytes)(JNIEnv *env, jclass c, jbyteArray a, jint v) {
    jsize n = (*env)->GetArrayLength(env, a);
    jbyte *p = (*env)->GetByteArrayElements(env, a, NULL);
    for (jsize i = 0; i < n; i++) {
        p[i] = (jbyte)(v + i);
    }
    (*env)->ReleaseByteArrayElements(env, a, p, 0);
}

JNIEXPORT jdoubleArray JNICALL FN(makeDoubles)(JNIEnv *env, jclass c, jint n) {
    jdouble buf[16];
    if (n > 16) {
        n = 16;
    }
    for (int i = 0; i < n; i++) {
        buf[i] = i * 0.5;
    }
    jdoubleArray r = (*env)->NewDoubleArray(env, n);
    (*env)->SetDoubleArrayRegion(env, r, 0, n, buf);
    return r;
}

JNIEXPORT jlong JNICALL FN(sumLongs)(JNIEnv *env, jclass c, jlongArray a) {
    jlong buf[16];
    jsize n = (*env)->GetArrayLength(env, a);
    if (n > 16) {
        n = 16;
    }
    (*env)->GetLongArrayRegion(env, a, 0, n, buf);
    jlong s = 0;
    for (jsize i = 0; i < n; i++) {
        s += buf[i];
    }
    return s;
}

static int cmp_int(const void *a, const void *b) {
    int x = *(const int *)a;
    int y = *(const int *)b;
    return (x > y) - (x < y);
}

JNIEXPORT jint JNICALL FN(sortFirstLast)(JNIEnv *env, jclass c, jintArray a) {
    jsize n = (*env)->GetArrayLength(env, a);
    int *copy = (int *)malloc(sizeof(int) * (n > 0 ? n : 1));
    (*env)->GetIntArrayRegion(env, a, 0, n, copy);
    qsort(copy, n, sizeof(int), cmp_int);
    int r = copy[0] * 1000 + copy[n - 1];
    free(copy);
    return r;
}

JNIEXPORT jbooleanArray JNICALL FN(flags)(JNIEnv *env, jclass c, jint n) {
    jboolean buf[16];
    if (n > 16) {
        n = 16;
    }
    for (int i = 0; i < n; i++) {
        buf[i] = (i % 3) == 0;
    }
    jbooleanArray r = (*env)->NewBooleanArray(env, n);
    (*env)->SetBooleanArrayRegion(env, r, 0, n, buf);
    return r;
}

JNIEXPORT jfloat JNICALL FN(sumFloats)(JNIEnv *env, jclass c, jfloatArray a) {
    jsize n = (*env)->GetArrayLength(env, a);
    jfloat *p = (jfloat *)(*env)->GetPrimitiveArrayCritical(env, a, NULL);
    jfloat s = 0;
    for (jsize i = 0; i < n; i++) {
        s += p[i];
    }
    (*env)->ReleasePrimitiveArrayCritical(env, a, p, JNI_ABORT);
    return s;
}

JNIEXPORT void JNICALL FN(negShorts)(JNIEnv *env, jclass c, jshortArray a) {
    jsize n = (*env)->GetArrayLength(env, a);
    jshort *p = (*env)->GetShortArrayElements(env, a, NULL);
    for (jsize i = 0; i < n; i++) {
        p[i] = (jshort)-p[i];
    }
    (*env)->ReleaseShortArrayElements(env, a, p, 0);
}

JNIEXPORT jobjectArray JNICALL FN(words)(JNIEnv *env, jclass c, jstring csv) {
    const char *s = (*env)->GetStringUTFChars(env, csv, NULL);
    int n = 1;
    for (const char *q = s; *q; q++) {
        if (*q == ',') {
            n++;
        }
    }
    jclass sc = (*env)->FindClass(env, "java/lang/String");
    jobjectArray r = (*env)->NewObjectArray(env, n, sc, NULL);
    const char *start = s;
    for (int i = 0; i < n; i++) {
        const char *end = strchr(start, ',');
        size_t len = end ? (size_t)(end - start) : strlen(start);
        char buf[64];
        if (len >= sizeof buf) {
            len = sizeof buf - 1;
        }
        memcpy(buf, start, len);
        buf[len] = 0;
        jstring w = (*env)->NewStringUTF(env, buf);
        (*env)->SetObjectArrayElement(env, r, i, w);
        (*env)->DeleteLocalRef(env, w);
        start = end ? end + 1 : start + len;
    }
    (*env)->ReleaseStringUTFChars(env, csv, s);
    return r;
}

JNIEXPORT jint JNICALL FN(countNonNull)(JNIEnv *env, jclass c, jobjectArray a) {
    jsize n = (*env)->GetArrayLength(env, a);
    int cnt = 0;
    for (jsize i = 0; i < n; i++) {
        jobject o = (*env)->GetObjectArrayElement(env, a, i);
        if (o != NULL) {
            cnt++;
        }
        (*env)->DeleteLocalRef(env, o);
    }
    return cnt;
}

JNIEXPORT jint JNICALL FN(callTwice)(JNIEnv *env, jclass c, jint x) {
    jmethodID m = (*env)->GetStaticMethodID(env, c, "twice", "(I)I");
    return (*env)->CallStaticIntMethod(env, c, m, x) + 1;
}

JNIEXPORT jstring JNICALL FN(fmt)(JNIEnv *env, jclass c, jdouble d, jlong l, jint i) {
    char buf[160];
    snprintf(buf, sizeof buf, "%.3f|%lld|%05d|%x|%s|%c|%5.1f|%e", d, (long long)l, i, i, "str", 'q', d, d);
    return (*env)->NewStringUTF(env, buf);
}

JNIEXPORT jstring JNICALL FN(libc)(JNIEnv *env, jclass c, jstring js) {
    const char *s = (*env)->GetStringUTFChars(env, js, NULL);
    char up[64];
    size_t n = strlen(s);
    if (n >= sizeof up) {
        n = sizeof up - 1;
    }
    for (size_t i = 0; i < n; i++) {
        up[i] = (char)toupper((unsigned char)s[i]);
    }
    up[n] = 0;
    char out[160];
    snprintf(out, sizeof out, "%s:%zu:%d:%ld:%d", up, strlen(s), atoi("1234"), strtol("ff", NULL, 16), strcmp("abc", "abd") < 0);
    (*env)->ReleaseStringUTFChars(env, js, s);
    return (*env)->NewStringUTF(env, out);
}

JNIEXPORT void JNICALL FN(thrower)(JNIEnv *env, jclass c, jstring msg) {
    jclass ex = (*env)->FindClass(env, "java/lang/IllegalStateException");
    const char *m = (*env)->GetStringUTFChars(env, msg, NULL);
    (*env)->ThrowNew(env, ex, m);
    (*env)->ReleaseStringUTFChars(env, msg, m);
}

JNIEXPORT jint JNICALL FN(catcher)(JNIEnv *env, jclass c) {
    jmethodID m = (*env)->GetStaticMethodID(env, c, "boom", "()V");
    (*env)->CallStaticVoidMethod(env, c, m);
    if ((*env)->ExceptionCheck(env)) {
        jthrowable t = (*env)->ExceptionOccurred(env);
        (*env)->ExceptionClear(env);
        jclass rc = (*env)->FindClass(env, "java/lang/RuntimeException");
        return (*env)->IsInstanceOf(env, t, rc) ? 1 : 2;
    }
    return 0;
}

JNIEXPORT void JNICALL FN(propagate)(JNIEnv *env, jclass c) {
    jmethodID m = (*env)->GetStaticMethodID(env, c, "boom", "()V");
    (*env)->CallStaticVoidMethod(env, c, m);
}

JNIEXPORT jobject JNICALL FN(makePoint)(JNIEnv *env, jclass c, jint x, jint y) {
    jclass pc = (*env)->FindClass(env, "com/winrt/nativetest/Point");
    jmethodID ctor = (*env)->GetMethodID(env, pc, "<init>", "(II)V");
    return (*env)->NewObject(env, pc, ctor, x, y);
}

JNIEXPORT jintArray JNICALL FN(virt)(JNIEnv *env, jclass c, jobject b) {
    jclass bc = (*env)->FindClass(env, "com/winrt/nativetest/Base");
    jmethodID m = (*env)->GetMethodID(env, bc, "value", "()I");
    jint r[2];
    r[0] = (*env)->CallIntMethod(env, b, m);
    r[1] = (*env)->CallNonvirtualIntMethod(env, b, bc, m);
    jintArray out = (*env)->NewIntArray(env, 2);
    (*env)->SetIntArrayRegion(env, out, 0, 2, r);
    return out;
}

JNIEXPORT jint JNICALL FN(classes)(JNIEnv *env, jclass c) {
    jclass d = (*env)->FindClass(env, "com/winrt/nativetest/Derived");
    jclass b = (*env)->FindClass(env, "com/winrt/nativetest/Base");
    int r = 0;
    if ((*env)->IsSameObject(env, (*env)->GetSuperclass(env, d), b)) {
        r |= 1;
    }
    if ((*env)->IsAssignableFrom(env, d, b)) {
        r |= 2;
    }
    if (!(*env)->IsAssignableFrom(env, b, d)) {
        r |= 4;
    }
    return r;
}

JNIEXPORT jint JNICALL FN(tlsNext)(JNIEnv *env, jclass c) {
    return ++tls_counter;
}

JNIEXPORT jint JNICALL FN(initValue)(JNIEnv *env, jclass c) {
    return init_value;
}

JNIEXPORT void JNICALL FN(keep)(JNIEnv *env, jclass c, jobject o) {
    if (g_kept != NULL) {
        (*env)->DeleteGlobalRef(env, g_kept);
    }
    g_kept = (*env)->NewGlobalRef(env, o);
}

JNIEXPORT jobject JNICALL FN(kept)(JNIEnv *env, jclass c) {
    return (*env)->NewLocalRef(env, g_kept);
}

JNIEXPORT jboolean JNICALL FN(vmCheck)(JNIEnv *env, jclass c) {
    JavaVM *vm = NULL;
    if ((*env)->GetJavaVM(env, &vm) != 0) {
        return 0;
    }
    JNIEnv *e2 = NULL;
    if ((*vm)->GetEnv(vm, (void **)&e2, JNI_VERSION_1_6) != 0) {
        return 0;
    }
    return vm == g_vm && e2 == env;
}

JNIEXPORT jboolean JNICALL FN(missingClass)(JNIEnv *env, jclass c) {
    jclass k = (*env)->FindClass(env, "does/not/Exist");
    jboolean pending = (*env)->ExceptionCheck(env);
    (*env)->ExceptionClear(env);
    return k == NULL && pending;
}

JNIEXPORT jboolean JNICALL FN(sync)(JNIEnv *env, jclass c, jobject o) {
    return (*env)->MonitorEnter(env, o) == 0 && (*env)->MonitorExit(env, o) == 0;
}

JNIEXPORT void JNICALL FN(bump)(JNIEnv *env, jobject self) {
    jclass c = (*env)->GetObjectClass(env, self);
    jfieldID fc = (*env)->GetFieldID(env, c, "counter", "I");
    (*env)->SetIntField(env, self, fc, (*env)->GetIntField(env, self, fc) + 1);
    jfieldID ft = (*env)->GetStaticFieldID(env, c, "total", "I");
    (*env)->SetStaticIntField(env, c, ft, (*env)->GetStaticIntField(env, c, ft) + 10);
    jfieldID fl = (*env)->GetFieldID(env, c, "label", "Ljava/lang/String;");
    (*env)->SetObjectField(env, self, fl, (*env)->NewStringUTF(env, "bumped"));
    jfieldID fb = (*env)->GetFieldID(env, c, "big", "J");
    (*env)->SetLongField(env, self, fb, (*env)->GetLongField(env, self, fb) * 2 + 1);
    jfieldID fr = (*env)->GetFieldID(env, c, "ratio", "D");
    (*env)->SetDoubleField(env, self, fr, (*env)->GetDoubleField(env, self, fr) * 1.5);
    jfieldID ff = (*env)->GetFieldID(env, c, "fl", "F");
    (*env)->SetFloatField(env, self, ff, (*env)->GetFloatField(env, self, ff) + 0.5f);
    jfieldID fz = (*env)->GetFieldID(env, c, "flag", "Z");
    (*env)->SetBooleanField(env, self, fz, !(*env)->GetBooleanField(env, self, fz));
    jfieldID fy = (*env)->GetFieldID(env, c, "by", "B");
    (*env)->SetByteField(env, self, fy, (jbyte)((*env)->GetByteField(env, self, fy) + 1));
    jfieldID fh = (*env)->GetFieldID(env, c, "ch", "C");
    (*env)->SetCharField(env, self, fh, (jchar)((*env)->GetCharField(env, self, fh) + 1));
    jfieldID fs = (*env)->GetFieldID(env, c, "sh", "S");
    (*env)->SetShortField(env, self, fs, (jshort)((*env)->GetShortField(env, self, fs) - 1));
}

static jmethodID describe_id(JNIEnv *env, jobject self) {
    jclass c = (*env)->GetObjectClass(env, self);
    return (*env)->GetMethodID(env, c, "describe", "(IJDFLjava/lang/String;ZCBS)Ljava/lang/String;");
}

JNIEXPORT jstring JNICALL FN(callMix)(JNIEnv *env, jobject self) {
    jmethodID m = describe_id(env, self);
    jstring s = (*env)->NewStringUTF(env, "str");
    return (jstring)(*env)->CallObjectMethod(env, self, m, 7, 1234567890123LL, 2.5, 1.25f, s, JNI_TRUE, 'x', (jbyte)-3, (jshort)-300);
}

JNIEXPORT jstring JNICALL FN(callMixA)(JNIEnv *env, jobject self) {
    jmethodID m = describe_id(env, self);
    jvalue a[9];
    a[0].i = 7;
    a[1].j = 1234567890123LL;
    a[2].d = 2.5;
    a[3].f = 1.25f;
    a[4].l = (*env)->NewStringUTF(env, "str");
    a[5].z = JNI_TRUE;
    a[6].c = 'x';
    a[7].b = -3;
    a[8].s = -300;
    return (jstring)(*env)->CallObjectMethodA(env, self, m, a);
}

static jobject call_v(JNIEnv *env, jobject o, jmethodID m, ...) {
    va_list ap;
    va_start(ap, m);
    jobject r = (*env)->CallObjectMethodV(env, o, m, ap);
    va_end(ap);
    return r;
}

JNIEXPORT jstring JNICALL FN(callMixV)(JNIEnv *env, jobject self) {
    jmethodID m = describe_id(env, self);
    jstring s = (*env)->NewStringUTF(env, "str");
    return (jstring)call_v(env, self, m, 7, 1234567890123LL, 2.5, 1.25f, s, JNI_TRUE, 'x', (jbyte)-3, (jshort)-300);
}
