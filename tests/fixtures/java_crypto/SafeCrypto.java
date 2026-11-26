import java.security.KeyPairGenerator;
import java.security.Signature;
import javax.crypto.Cipher;

class SafeCrypto {
    void inspect() throws Exception {
        KeyPairGenerator.getInstance("ML-KEM-768");
        Signature.getInstance("ML-DSA-65");
        Cipher.getInstance("AES_256/GCM/NoPadding");
    }
}
