# Serviço Linux/systemd

Exemplo de unidade systemd:

```ini
[Unit]
Description=Manifestador NF-e Server
After=network.target

[Service]
ExecStart=/opt/manifestador-nfe/app-server --mode=headless-api
Restart=always
User=ManifestadorNFe
Environment=APP_ENV=production

[Install]
WantedBy=multi-user.target
```
