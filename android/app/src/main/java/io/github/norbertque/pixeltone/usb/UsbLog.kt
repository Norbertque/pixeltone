package io.github.norbertque.pixeltone.usb

import android.util.Log

/** Logcat for USB output, printf-style. Field reports of new DACs start here. */
internal object UsbLog {
    fun d(tag: String, message: String, vararg args: Any?) {
        Log.d(tag, message.format(*args))
    }

    fun i(tag: String, message: String, vararg args: Any?) {
        Log.i(tag, message.format(*args))
    }

    fun w(tag: String, message: String, vararg args: Any?) {
        Log.w(tag, message.format(*args))
    }
}
