import javax.crypto.*;
import javax.crypto.spec.*;

class Symmetric {
    void inspect() throws Exception {
        Cipher.getInstance("RSA/ECB/OAEPWithSHA-256AndMGF1Padding"); // @expect rsa:encryption
        Cipher.getInstance("DESede/CBC/PKCS5Padding"); // @expect 3des:encryption
        Cipher.getInstance("ChaCha20-Poly1305"); // @expect chacha20:encryption
        Cipher.getInstance("AES_192/GCM/NoPadding"); // @expect aes-192:encryption
        var generator = KeyGenerator.getInstance("AES"); // @expect aes-128:encryption
        generator.init(128);
        var key = generator.generateKey();
        var cipher = Cipher.getInstance("AES/GCM/NoPadding"); // @expect aes-128:encryption
        cipher.init(Cipher.ENCRYPT_MODE, key);
        new SecretKeySpec(new byte[32], "AES"); // @expect aes-256:encryption
    }
}
