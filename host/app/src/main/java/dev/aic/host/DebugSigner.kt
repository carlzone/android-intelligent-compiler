package dev.aic.host

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import com.android.apksig.ApkSigner
import java.io.File
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.PrivateKey
import java.security.cert.X509Certificate
import javax.security.auth.x500.X500Principal

/** One local identity; never exported in project archives. */
object DebugSigner {
    private const val ALIAS = "aic-generated-debug-v1"
    @Synchronized fun sign(input: File, output: File) {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        if (!store.containsAlias(ALIAS)) {
            KeyPairGenerator.getInstance(KeyProperties.KEY_ALGORITHM_RSA, "AndroidKeyStore").apply {
                initialize(KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_VERIFY)
                    .setKeySize(2048)
                    .setDigests(KeyProperties.DIGEST_SHA256, KeyProperties.DIGEST_SHA512)
                    .setSignaturePaddings(KeyProperties.SIGNATURE_PADDING_RSA_PKCS1)
                    .setCertificateSubject(X500Principal("CN=AIC Local Debug"))
                    .build())
            }.generateKeyPair()
        }
        val config = ApkSigner.SignerConfig.Builder("aic-debug", store.getKey(ALIAS, null) as PrivateKey,
            listOf(store.getCertificate(ALIAS) as X509Certificate)).build()
        ApkSigner.Builder(listOf(config)).setInputApk(input).setOutputApk(output)
            .setMinSdkVersion(23).setV1SigningEnabled(true).setV2SigningEnabled(true)
            .setV3SigningEnabled(false).setV4SigningEnabled(false).build().sign()
    }
}
