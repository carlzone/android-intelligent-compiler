package dev.aic.host

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import org.json.JSONObject
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

class SecretStore(private val context: Context) {
    private val prefs=context.getSharedPreferences("ai-secrets",Context.MODE_PRIVATE)
    private val alias="dev.aic.host.ai.credentials.v1"
    private fun key(): SecretKey {
        val store=KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(alias,null) as? SecretKey)?.let { return it }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES,"AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(alias,KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())
        }.generateKey()
    }
    fun saveCredential(provider: String, value: String) {
        if(value.isBlank()) { prefs.edit().remove(provider).apply(); return }
        val cipher=Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE,key()) }
        val data=cipher.doFinal(value.toByteArray())
        prefs.edit().putString(provider,JSONObject().put("iv",Base64.encodeToString(cipher.iv,Base64.NO_WRAP))
            .put("data",Base64.encodeToString(data,Base64.NO_WRAP)).toString()).apply()
    }
    fun credential(provider: String): String {
        val raw=prefs.getString(provider,null) ?: return ""
        return runCatching {
            val json=JSONObject(raw); val cipher=Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.DECRYPT_MODE,key(),GCMParameterSpec(128,Base64.decode(json.getString("iv"),Base64.NO_WRAP)))
            String(cipher.doFinal(Base64.decode(json.getString("data"),Base64.NO_WRAP)))
        }.getOrDefault("")
    }
    fun saveApiKey(value: String)=saveCredential(ProviderKind.OPENAI.id,value)
    fun apiKey(): String=credential(ProviderKind.OPENAI.id)
}
