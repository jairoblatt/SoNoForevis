# Só no Forevis

O primeiro load balancer full io_uring do mundo que não serve pra porra nenhuma.

Proxy TCP → UDS → TCP de ultra alta performance escrito em Rust com [Monoio](https://github.com/bytedance/monoio). Encaminha bytes. Só isso.

## Como funciona

```
cliente TCP → [SoNoForevis] → backend UDS
                    ↑
             round-robin puro
             sem estado global
             sem locks
             sem overhead
             sem utilidade prática
```

- Listener TCP em porta configurável
- Backends via Unix Domain Socket
- Encaminhamento bidirecional puro de bytes — zero parsing, zero protocolo
- Round-robin por core sem atomics
- Thread-per-core: 1 runtime io_uring por CPU
- `SO_REUSEPORT`: o kernel distribui as conexões entre as threads
- `TCP_NODELAY`: sem Nagle, sem latência extra
- Buffer de 64 KB pré-alocado por direção por conexão, reutilizado em loop

## Variáveis de ambiente

| Variável   | Obrigatório | Padrão  | Descrição                                      |
|------------|-------------|---------|------------------------------------------------|
| `UPSTREAMS`| Sim         | —       | Caminhos UDS separados por vírgula             |
| `PORT`     | Não         | `8080`  | Porta TCP de entrada                           |
| `BUF_SIZE` | Não         | `65536` | Tamanho do buffer por direção em bytes (64 KB) |

## Uso

```bash
PORT=9999 UPSTREAMS=/run/sock/api1.sock,/run/sock/api2.sock ./SoNoForevis
```

## Docker

```bash
docker build -t so-no-forevis .
docker run --security-opt seccomp:unconfined \
  -e PORT=9999 \
  -e UPSTREAMS=/run/sock/api1.sock,/run/sock/api2.sock \
  -p 9999:9999 \
  so-no-forevis
```

> `seccomp:unconfined` é obrigatório — o Docker bloqueia `io_uring_setup` por padrão.

## Requisitos

- Linux com kernel ≥ 5.1 (io_uring)
- Rust edition 2024

## Por que io_uring?

Porque `read`/`write` normais fazem syscall por operação. io_uring submete um lote de operações no submission ring e coleta os resultados no completion ring — menos transições user/kernel, menos overhead, mais throughput.

## Por que não serve pra nada?

Serve. Só que exclusivamente como proxy de bytes sobre UDS. Sem TLS, sem HTTP, sem métricas, sem retry, sem health check, sem service discovery. Se você precisa de qualquer uma dessas coisas, use outro proxy.

Se você só quer empurrar bytes o mais rápido possível entre um TCP e um UDS, esse é o seu negócio.

## Licença

[Mexe no Forévis](./LICENSE) — pode usar, modificar e distribuir. Só não pode dizer que não foi avisado.
