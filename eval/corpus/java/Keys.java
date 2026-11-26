import java.security.*;
import java.security.spec.*;
import javax.crypto.*;

class Keys {
    void inspect() throws Exception {
        var rsa = KeyPairGenerator.getInstance("RSA"); // @expect rsa:unknown
        rsa.initialize(2048);
        var dsa = KeyPairGenerator.getInstance("DSA"); // @expect dsa:signature
        dsa.initialize(2048);
        var ec = KeyPairGenerator.getInstance("EC"); // @expect ecdh:key_exchange
        ec.initialize(new ECGenParameterSpec("secp256r1"));
        KeyAgreement.getInstance("ECDH"); // @expect ecdh:key_exchange
        KeyPairGenerator.getInstance("DiffieHellman"); // @expect dh:key_exchange
        KeyAgreement.getInstance("DH"); // @expect dh:key_exchange
        KeyAgreement.getInstance("X25519"); // @expect x25519:key_exchange
        var x = KeyPairGenerator.getInstance("XDH"); // @expect x25519:key_exchange
        x.initialize(NamedParameterSpec.X25519);
        KeyPairGenerator.getInstance("Ed25519"); // @expect ed25519:signature
        KeyPairGenerator.getInstance("ML-KEM-768"); // @expect ml-kem:key_exchange
        
        KEM.getInstance("ML-KEM"); // @expect ml-kem:key_exchange
        KeyFactory.getInstance("ML-DSA"); // @expect ml-dsa:signature
    }
}
