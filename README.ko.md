# herdr-sheep

*[English README](README.md)*

[Herdr](https://herdr.dev) 세션에서 돌고 있는 코딩 에이전트를 전부 양으로 보여주는
플러그인 pane입니다. 양은 담당 에이전트의 상태에 따라 뛰어다니고, 풀을 뜯고,
점프하고, 도움을 요청하고, 코를 골기 때문에 pane을 한 번 흘겨보는 것만으로
어떤 에이전트가 나를 기다리는지 알 수 있습니다.

```text
(>_@ herdr-sheep      1 waiting on you  1 done  2 working  1 idle  1 unknown
~ GATE  1 waiting on you ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
                                   (?)
                                   ,@~~~.
                                  (o_ ~~ )
                                   ''  ''
                                   deploy
                   ,     ', '    .           .        ' .      '     .
~ PEN  1 done ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
                                        +
                                   ,@~~~. *
                                 +(^_ ~~ )
                                   \'  '/
                                  reviewer
 '             ,           ,          ,    '   ,  . .   '    .           ,
~ PADDOCK  2 working ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
                           ,@~~~.                                   ,~~~@.
                          (>_ ~~ )                                 ( ~~ _<)
                           /'  '\                                   /'  '\
                          *runner                                    docs
      '  ,''                       .       .      '    '    ,            , .
~ MEADOW  1 idle ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
                                 ,~~~@.
                                 ( ~~ )
                               , ''  _< .
                                 scout
~ FOLD  1 unknown ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
                                        z z z
                                   ,@~~~.
                                  (-_ ~~ )
                                   ~~~~~~
                                  mystery
press j or k to pick a sheep
j/k select  enter focus  r refresh  q quit
```

Herdr 마스코트는 뿔이 말린 측면 프로필의 양이고 얼굴이 셸 프롬프트라서,
모든 양이 `@` 뿔과 `>_` 얼굴을 유지합니다.

## 양이 하는 일

양은 에이전트 상태별 구역(zone)에 서 있고, 구역은 "지금 나를 얼마나 필요로
하는가" 순서로 위에서부터 배치됩니다. pane 맨 위에 있는 것이 먼저 봐야 할
것입니다. 해당 상태의 에이전트가 없는 구역은 그리지 않습니다.

| Herdr 상태 | 구역 | 양의 행동 |
| --- | --- | --- |
| `blocked` | `GATE` | 멈춰 서서 `(?)`를 깜빡이며 승인이나 답변을 기다립니다 |
| `done` | `PEN` | 반짝이며 점프합니다 |
| `working` | `PADDOCK` | 자기 레인을 좌우로 질주하며 먼지를 냅니다 |
| `idle` | `MEADOW` | 고개를 들었다 숙이며 풀을 뜯고 조금씩 움직입니다 |
| `unknown` | `FOLD` | 엎드려서 `z z z` 코를 골며 잡니다 |

양털 색은 에이전트가 보고하는 모델 제공자(`claude`, `codex`, `gemini` 등)에서
가져오고, 없으면 Herdr가 감지한 에이전트 종류를 씁니다. 이름은 `herdr agent
rename`으로 지정한 이름을 우선 쓰고, 없으면 에이전트가 보고하는 짧은 제목을
씁니다. 이름 앞의 `*`는 Herdr가 현재 포커스하고 있는 pane이라는 표시입니다.

상태가 바뀌면 양은 새 구역으로 걸어갑니다. 새로 뜬 에이전트의 양은 왼쪽
바깥에서 걸어 들어오고, 종료된 에이전트의 양은 무리에서 빠집니다.

## 설치

요구사항: Herdr 0.8.0 이상, 그리고 머신에 `cargo`. 이 플러그인은 Rust 바이너리라서
Herdr가 설치 시점에 빌드합니다.

```bash
herdr plugin install huketo/herdr-sheep
herdr plugin pane open --plugin huketo.sheep --entrypoint pasture
```

`~/.config/herdr/config.toml`에 아래를 넣으면 키에 바인딩됩니다.

```toml
[[keys.command]]
key = "prefix+p"
type = "plugin_action"
command = "huketo.sheep.open-pasture"
description = "sheep pasture"
```

이 액션은 키를 누른 pane 옆에 목장을 split으로 엽니다.

## 키

| 키 | 동작 |
| --- | --- |
| `j` `k`, 방향키, `Tab` | 구역 순서대로 양 사이를 이동 |
| `Enter` `f` | 선택한 양의 에이전트 pane으로 포커스 이동. 선택이 없으면 가장 급한 에이전트로 이동 |
| `r` | 다음 폴링을 기다리지 않고 즉시 세션 상태를 다시 읽기 |
| `q` `Esc` `Ctrl-C` | 목장을 나가고 pane을 닫기 |

양을 선택하면 키 힌트 바로 위 줄에 pane id, 워크스페이스, 에이전트 종류와
제공자, 컨텍스트 사용량, 사용량 한도, 터미널 타이틀이 표시됩니다.

## 좁은 pane

목장을 그리기에 너무 낮거나 좁은 pane은 에이전트 한 줄씩의 목록으로 바뀝니다.
순서는 그대로 급한 것부터이고, 못 들어간 에이전트 수를 알려줍니다. pane을 키우면
양들이 돌아옵니다.

## 설정

| 환경 변수 | 기본값 | 의미 |
| --- | --- | --- |
| `HERDR_SHEEP_FPS` | `20` | 애니메이션 초당 프레임 수, 5~60으로 제한 |
| `HERDR_SHEEP_POLL_MS` | `800` | 세션 상태 폴링 주기(ms), 200~10000으로 제한 |

상태는 `HERDR_BIN_PATH`를 통해 `herdr api snapshot`으로 읽고, 포커스 이동은
`herdr agent focus`로 합니다. 그 외에는 아무것도 쓰지 않고 상태도 저장하지 않습니다.

## pane 밖에서 실행하기

```bash
herdr-sheep --snapshot 100x34    # 현재 세션의 한 프레임을 일반 텍스트로 출력
herdr-sheep --demo               # 모든 상태의 양이 한 마리씩 있는 프레임 출력
herdr-sheep --help
```

`--snapshot`은 Herdr 서버가 떠 있어야 하고, `--demo`는 필요 없습니다.

## 개발

```bash
git clone https://github.com/huketo/herdr-sheep
cd herdr-sheep
cargo build --release
herdr plugin link "$PWD"
herdr plugin pane open --plugin huketo.sheep --entrypoint pasture
```

`plugin link`은 build 커맨드를 실행하지 않으므로 코드를 고친 뒤에는 직접 빌드하고
pane을 다시 열어야 합니다. `cargo test`는 스냅샷 파서, 구역 배치, 스프라이트 미러링,
애니메이션, 렌더 결과를 검사합니다.

## 라이선스

MIT. [LICENSE](LICENSE) 참고.
