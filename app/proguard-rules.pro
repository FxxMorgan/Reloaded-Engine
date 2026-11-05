# Proguard rules for Pokemon Reloaded Android
-keep class com.reloaded.android.bridge.** { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}
