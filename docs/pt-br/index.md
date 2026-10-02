---
title: Contas de Usuário
description: Gerenciar perfis de usuário, nomes de exibição, avatares e grupos.
slug: pt/0.4.0/docs/user-guide/sessions/accounts
---

`argvus-accounts` é o gerenciador oficial de contas para a área de trabalho ARGVUS. Ele gerencia perfis de usuários locais sem exigir privilégios administrativos para a maioria das operações — nomes de exibição, avatares e associações de grupos são tratados através do sistema nativo de autorização da sua área de trabalho.

## Listar usuários

```sh
argvus-accounts list              # Mostrar usuários
argvus-accounts list --all        # Incluir contas de sistema
```

## Visualizar informações de conta

```sh
argvus-accounts show <username>   # Exibir detalhes da conta
argvus-accounts self              # Mostrar sua própria conta
argvus-accounts self show         # Equivalente a 'self'
```

A saída inclui nome de usuário, UID, GID, diretório inicial, shell, associações de grupo e localização do avatar.

## Gerenciar nome de exibição

Altere como seu nome aparece no inicializador gráfico e na área de trabalho:

```sh
argvus-accounts self name "Seu Nome"              # Alterar seu próprio nome
argvus-accounts name <user> "Nome de Exibição"   # Admin: alterar nome de outro usuário
```

Os nomes de exibição são armazenados no campo GECOS padrão (o campo `comment` do Unix). O nome de usuário nunca é modificado.

## Gerenciar avatares

Os avatares aparecem no inicializador gráfico e em áreas de perfil do usuário. O arquivo deve ser uma imagem (PNG, JPEG ou WebP). É automaticamente redimensionado para 256×256 pixels e convertido para PNG.

```sh
# Definir ou substituir seu avatar
argvus-accounts self avatar ~/Pictures/profile-pic.png

# Remover seu avatar
argvus-accounts self avatar --remove

# Admin: definir avatar de outro usuário
argvus-accounts avatar <user> ~/Pictures/avatar.png
argvus-accounts avatar <user> --remove
```

Os avatares são armazenados em `~/.face` seguindo a convenção padrão do freedesktop. Esta localização também funciona com LightDM, SDDM e outros gerenciadores de exibição.

**Requisitos de avatar**:

* Formatos suportados: PNG, JPEG, WebP
* Tamanho máximo de entrada: 20 MiB
* Saída: sempre 256×256 PNG com modo 0644

## Gerenciar associação de grupo

Visualize e modifique quais grupos um usuário pertence. Isso pode exigir autorização de administrador.

```sh
# Visualizar associação de grupo
argvus-accounts groups <user>
argvus-accounts self groups

# Adicionar a um grupo (requer autorização)
argvus-accounts groups <user> --add wheel
argvus-accounts groups <user> --add audio --add video

# Remover de um grupo (requer autorização)
argvus-accounts groups <user> --remove wheel
```

Grupos comuns incluem `wheel` (acesso de administrador), `audio` (acesso de som), `video` (aceleração de gráficos) e `input` (manipulação de teclado/mouse).

## Alterar senha

Altere sua própria senha ou (se você for um administrador) redefina a senha de outro usuário.

```sh
# Alterar sua própria senha (senha atual necessária como prova)
argvus-accounts passwd <user> <old-password> <new-password> <confirm-password>

# Redefinição de senha do administrador (senha atual é ignorada)
sudo argvus-accounts passwd <user> ignored <new-password> <confirm-password>
```

A senha de confirmação deve corresponder exatamente à nova senha. Embora digitada na linha de comando aqui, as senhas devem idealmente ser inseridas interativamente (uma melhoria futura). Os administradores podem redefinir senhas sem saber a atual; usuários regulares devem provar que lembram sua senha atual antes de alterá-la.

## Autorização

A maioria dos comandos `argvus-accounts` funciona sem privilégios de administrador:

| Operação | Usuário regular | Administrador |
| --- | --- | --- |
| Listar usuários | ✓ | ✓ |
| Visualizar informações de conta | ✓ | ✓ |
| Alterar nome de exibição próprio | ✓ | ✓ |
| Alterar avatar próprio | ✓ | ✓ |
| Alterar própria senha | ✓ (com prova) | ✓ |
| Modificar outros usuários | — | ✓ |
| Alterar grupos | — | ✓ |
| Redefinir senhas de outros | — | ✓ |

Quando um usuário regular tenta uma operação administrativa (como alterar o nome de outro usuário), o agente de autorização da sua área de trabalho solicita permissão. Não é necessário usar `sudo` — `argvus-accounts` solicita automaticamente elevação através do mecanismo PolicyKit padrão do sistema.

## Modo verbose

Para diagnósticos, adicione `--verbose` a qualquer comando:

```sh
argvus-accounts --verbose list
argvus-accounts --verbose self avatar ~/Pictures/avatar.png
```

Isso imprime informações detalhadas sobre o que o comando está fazendo.

## Exemplos

**Configure seu perfil após o login**:

```sh
# Defina seu nome de exibição
argvus-accounts self name "William Canin"

# Faça upload de um avatar
argvus-accounts self avatar ~/Pictures/profile.png
```

**Gerencie associação de grupo**:

```sh
# Verifique seus grupos atuais
argvus-accounts self groups

# Solicite para aderir a um grupo (se seu administrador permitir)
argvus-accounts groups $USER --add wheel
```

**Tarefas de administrador de sistema**:

```sh
# Listar todos os usuários e contas de sistema
argvus-accounts list --all

# Verificar grupos e avatar de um usuário
argvus-accounts show ghost
argvus-accounts groups ghost

# Atualizar perfil de um usuário
argvus-accounts avatar ghost ~/Desktop/ghost-profile.png
argvus-accounts name ghost "Ghost Account"
```
