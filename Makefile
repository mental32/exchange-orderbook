
ssl:
	openssl req -new -x509 -nodes -subj "/CN=my.root" -newkey rsa:2048 -keyout ./etc/letsencrypt/ca.key -out ./etc/letsencrypt/ca.crt -reqexts v3_req -extensions v3_ca
	openssl req -new -nodes -sha256 -newkey rsa:2048 -keyout ./etc/letsencrypt/domain.key -config ./etc/ext.conf -out ./etc/letsencrypt/domain.csr
	openssl x509 -req -in ./etc/letsencrypt/domain.csr -CA ./etc/letsencrypt/ca.crt -CAkey ./etc/letsencrypt/ca.key -CAcreateserial -out ./etc/letsencrypt/domain.crt -days 500 -sha256 -extfile ./etc/ext.conf -extensions req_ext
	cp ./etc/letsencrypt/domain.key ./etc/letsencrypt/privkey.pem
	cat ./etc/letsencrypt/domain.crt ./etc/letsencrypt/ca.crt > ./etc/letsencrypt/fullchain.pem
