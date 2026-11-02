package io.github.norbertque.pixeltone.remote

import android.content.Context
import android.content.pm.PackageManager
import android.os.Build

/**
 * Whether Pixeltone may find and reach devices on the Wi-Fi: Android 17's local network
 * protection asks the user first (the permission is in the Nearby devices group); earlier
 * Android versions always let it.
 */
fun mayUseLocalNetwork(context: Context): Boolean =
    Build.VERSION.SDK_INT < 37 || context.checkSelfPermission(LOCAL_NETWORK_PERMISSION) == PackageManager.PERMISSION_GRANTED

/** The permission [mayUseLocalNetwork] needs from Android 17 on (`Manifest.permission.ACCESS_LOCAL_NETWORK`). */
const val LOCAL_NETWORK_PERMISSION = "android.permission.ACCESS_LOCAL_NETWORK"
