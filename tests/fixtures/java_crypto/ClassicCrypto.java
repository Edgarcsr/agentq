import java.security.KeyPairGenerator;
import java.security.MessageDigest;
import java.security.Signature;
import javax.crypto.Cipher;
import javax.crypto.KeyAgreement;
import javax.crypto.KeyGenerator;

class ClassicCrypto {
    void inspect() throws Exception {
        var rsa = KeyPairGenerator.getInstance("RSA");
        rsa.initialize(2048);
        var signer = Signature.getInstance("SHA256withRSA");
        signer.initSign(rsa.generateKeyPair().getPrivate());
        KeyAgreement.getInstance("ECDH");
        MessageDigest.getInstance("MD5");
        var aes = KeyGenerator.getInstance("AES");
        aes.init(128);
        var key = aes.generateKey();
        var cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.ENCRYPT_MODE, key);
    }
}
