from cryptography.hazmat.primitives.asymmetric import ec, rsa

private_key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
signature = private_key.sign(data, padding, algorithm)
ec_private_key = ec.generate_private_key(curve)
shared_secret = ec_private_key.exchange(ec.ECDH(), peer_public_key)