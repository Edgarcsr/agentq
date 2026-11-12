from cryptography.hazmat.primitives.asymmetric import padding, rsa

private = rsa.generate_private_key(public_exponent=65537, key_size=2048)  # @expect rsa:encryption
public = private.public_key()
public.encrypt(data, padding.OAEP(mgf=None, algorithm=None, label=None))