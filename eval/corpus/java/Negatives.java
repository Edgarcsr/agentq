import java.security.MessageDigest;
import java.security.Signature;
import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;

class Negatives {
    // RSA and MD5 in comments are not cryptographic calls.
    static final String LABEL = "SHA-1";

    void shadowed(Object MessageDigest) {
        MessageDigest.getInstance("MD5");
    }

    void dynamic(String algorithm) throws Exception {
        MessageDigest.getInstance(algorithm);
        String name = "MD5";
        name = System.getenv("HASH_ALGORITHM");
        MessageDigest.getInstance(name);
        Cipher.getInstance("AES/GCM/NoPadding"); // Key size is unknown.
        KeyGenerator.getInstance("AES"); // Provider default is not inferred.
        MessageDigest.getInstance("SHA-512/256"); // No matching knowledge-base entry.
        Signature.getInstance("Ed448"); // Not modeled in the knowledge base yet.
    }
}
