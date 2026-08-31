# herdr-sheep

*[English README](README.md)*

[Herdr](https://herdr.dev) 세션에서 돌고 있는 코딩 에이전트를 전부 양으로 보여주는
플러그인 pane입니다. 양은 담당 에이전트의 상태에 따라 뛰어다니고, 풀을 뜯고,
점프하고, 도움을 요청하고, 코를 골기 때문에 pane을 한 번 흘겨보는 것만으로
어떤 에이전트가 나를 기다리는지 알 수 있습니다.

```text
(>_@ herdr-sheep        1 waiting on you  1 done  2 working  1 idle  1 unknown
|- GATE  1 waiting on you [+]-|----|----|----|----|----|----|----|----|----|--
                                    (?)
                                    ,@~~~.
                                   (o_ ~~ )
                                    ''  ''
                                    deploy
                   ,     ', '    .           .        ' .      '     .       ,
|- PEN  1 done |----|----|----|----|----|----|----|----|----|----|----|----|--
                                         *
                                    ,@~~~. *
                                   (^_ ~~ )
                                    \'  '/
                                   reviewer
 '             ,           ,          ,    '   ,  . .   '    .           ,
|- PADDOCK  2 working ---|----|----|----|----|----|----|----|----|----|----|--
                     ,~~~@.                            ,~~~@.
                    ( ~~ _<)                          ( ~~ _<)
                  .  \'  '/                         .  \'  '/
                    *runner                             docs
      '  ,''                       .       .      '    '    ,            , .
|- MEADOW  1 idle --|----|----|----|----|----|----|----|----|----|----|----|--
                                  ,~~~@.
                                  ( ~~ )
                                , ''  _< .
                                  scout
   . '      .                . , .      '       ,                .   ,,   '
|- FOLD  1 unknown -|----|----|----|----|----|----|----|----|----|----|----|--
                                         z
                                    ,@~~~.
                                   (-_ ~~ )
                                    ~~~~~~
                                   mystery
         ,    .   '            .  ,         ,       ,   '        .   '
.       ,           ,                       .                   '   _____
 .               .                 ,                  .            /_____\
                       ,         ,                                /|  ^  |\
|----|----|----|----|----|----|----|----|----|----|----|----|----|-|_|X|_|-|--
press j or k to pick a sheep
j/k select  click/enter focus  r refresh  q quit
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

## 목장 풍경

구역은 울타리로 나뉜 방목장이라서, 구역 구분선 자체가 울타리이고, 나를 기다리는
에이전트가 모인 구역에는 이름 그대로 닫힌 문이 달려 있습니다. 모든 양이
설 자리를 받고도 행이 남는 pane은 아래쪽에 지평선 울타리가 생기고, 몇 행 더
남으면 그 울타리에 외양간이 함께 섭니다. 풍경이 양의 자리를 빼앗는 일은 없어서,
pane 높이가 무리에 딱 맞으면 울타리와 외양간은 그냥 그려지지 않습니다. 풀은
아무도 쓰지 않는 행에만 자라고, 무리에서 멀어질수록 옅어집니다.

## 설치

요구사항: Herdr 0.8.0 이상. 설치 시 플랫폼에 맞는 미리 빌드된 바이너리를 릴리스에서
받아오므로 Rust 툴체인은 필요하지 않습니다.

```bash
herdr plugin install huketo/herdr-sheep
herdr plugin pane open --plugin huketo.sheep --entrypoint pasture
```

미리 빌드된 바이너리는 macOS(Apple silicon, Intel)와 Linux(x86_64, aarch64 —
정적 musl 빌드)를 지원합니다.

## 키와 마우스

| 키 | 동작 |
| --- | --- |
| `j` `k`, 방향키, `Tab` | 구역 순서대로 양 사이를 이동 |
| `Enter` `f` | 선택한 양의 에이전트 pane으로 포커스 이동. 선택이 없으면 가장 급한 에이전트로 이동 |
| `r` | 다음 폴링을 기다리지 않고 즉시 세션 상태를 다시 읽기 |
| `q` `Esc` `Ctrl-C` | 목장을 나가고 pane을 닫기 |

| 마우스 | 동작 |
| --- | --- |
| 양 클릭 | 그 양을 선택 |
| 한 번 더 클릭 | 그 에이전트의 pane으로 포커스 이동 |

스프라이트 위가 아니라 그 양의 레인 안 아무 곳이나 클릭해도 됩니다. 양은 레인
안에서 계속 움직이는데, 그걸 맞히는 타이밍 게임이 되면 안 되니까요. 목장은
터미널에 버튼 입력만 요청하므로 pane 위에서 마우스를 움직이는 것 자체는 아무
비용도 들지 않습니다.

양을 선택하면 키 힌트 바로 위 줄에 pane id, 워크스페이스, 에이전트 종류와
제공자, 컨텍스트 사용량, 사용량 한도, 터미널 타이틀이 표시됩니다.

## 키에 바인딩하기

목장을 여는 이유가 두 가지라서 액션도 두 개입니다.

| 액션 | 여는 방식 |
| --- | --- |
| `huketo.sheep.open-here` | 키를 누른 pane 위에 오버레이로 엽니다. 목장을 닫으면 Herdr가 이전 레이아웃과 포커스를 되돌려 주므로, "무리 전체를 잠깐 본다"용입니다. |
| `huketo.sheep.toggle-side` | 그 pane 옆 split으로 열고 그대로 둡니다. 같은 키를 다시 누르면 어디에 있든 목장을 닫으므로 두 개가 생기지 않습니다. |

`~/.config/herdr/config.toml`에 아래를 넣습니다.

```toml
[[keys.command]]
key = "prefix+y"
type = "plugin_action"
command = "huketo.sheep.open-here"
description = "Sheep: pasture over this pane"

# 선택 사항: prefix+y와 같은 액션을 한 번 누르기로도.
[[keys.command]]
key = "f10"
type = "plugin_action"
command = "huketo.sheep.open-here"
description = "Sheep: pasture over this pane"

[[keys.command]]
key = "prefix+shift+y"
type = "plugin_action"
command = "huketo.sheep.toggle-side"
description = "Sheep: toggle pasture side pane"
```

그다음 `herdr server reload-config`를 실행하면 적용되고, `prefix+?`에 새 바인딩이
보입니다.

키는 취향대로 고르되 Herdr 기본 바인딩을 먼저 확인하세요. `prefix+s`는 설정,
`prefix+p`는 이전 탭이라 둘 다 비어 있지 않습니다. 여기서 `prefix+y`를 고른 건
`y`가 비어 있고 "양"의 첫 글자라서입니다. Kitty 키보드 프로토콜이 없는
터미널(예: Windows Terminal)에서는 `ctrl+shift+<문자>`가 shift 없이 도착하므로,
위의 직접 단축키는 조합키가 아니라 펑션키로 뒀습니다.

두 액션은 Herdr 커맨드 팔레트에서도, `herdr plugin action invoke
huketo.sheep.toggle-side`로도 실행할 수 있습니다.

## 좁은 pane

목장을 그리기에 너무 낮거나 좁은 pane은 에이전트 한 줄씩의 목록으로 바뀝니다.
순서는 그대로 급한 것부터이고, 못 들어간 에이전트 수를 알려줍니다. pane을 키우면
양들이 돌아옵니다. 목록에서도 줄을 클릭하면 그 에이전트가 선택됩니다.

## 설정

| 환경 변수 | 기본값 | 의미 |
| --- | --- | --- |
| `HERDR_SHEEP_FPS` | `20` | 애니메이션 초당 프레임 수, 5~60으로 제한 |
| `HERDR_SHEEP_POLL_MS` | `800` | 세션 상태 폴링 주기(ms), 200~10000으로 제한 |

상태는 `HERDR_BIN_PATH`를 통해 `herdr api snapshot`으로 읽고, 포커스 이동은
`herdr agent focus`, 자기 pane을 열고 닫는 것은 `herdr plugin pane`으로 합니다.
그 외에는 아무것도 쓰지 않고 상태도 저장하지 않습니다.

## pane 밖에서 실행하기

```bash
herdr-sheep --snapshot 100x34    # 현재 세션의 한 프레임을 일반 텍스트로 출력
herdr-sheep --demo               # 모든 상태의 양이 한 마리씩 있는 프레임 출력
herdr-sheep --open side          # 사이드 split 목장 토글
herdr-sheep --help
```

`--snapshot`과 `--open`은 Herdr 서버가 떠 있어야 하고, `--demo`는 필요 없습니다.

## 개발

```bash
git clone https://github.com/huketo/herdr-sheep
cd herdr-sheep
just link      # release 빌드 + bin/herdr-sheep 배치 + herdr plugin link .
```

`herdr plugin link`는 build 커맨드를 실행하지 않으므로, 링크된 체크아웃은 자기
바이너리를 매니페스트가 찾는 위치인 `bin/`에 직접 놓습니다. 코드를 고친 뒤에는
`just install`을 다시 실행하고 pane을 다시 여세요.

`just`를 실행하면 전체 태스크 목록이 나옵니다. 알아두면 좋은 것들:

| 태스크 | 하는 일 |
| --- | --- |
| `just ci` | CI가 하는 전부: 버전 일치 검사, `fmt --check`, `-D warnings` clippy, 테스트, 마우스 스모크 테스트, 프레임 렌더 |
| `just fmt` | 코드 포맷 |
| `just test` | 유닛 테스트 + `tests/cli.rs`의 종단 테스트 |
| `just smoke` | pty 스모크 테스트: 실제 바이너리를 스텁 `herdr`에 붙여 띄우고, 양을 클릭한 뒤 pane의 반응을 확인 |
| `just audit` | `deny.toml` 기준 `cargo deny check` |
| `just changelog` | 커밋 히스토리에서 `CHANGELOG.md` 재생성 |
| `just demo 120x40` | 지정한 크기로 프레임 한 장 출력 |

유닛 테스트는 스냅샷 파서, 구역 배치와 클릭 판정, 스프라이트 미러링, 애니메이션,
렌더 결과를 검사합니다. `tests/cli.rs`는 빌드된 바이너리를 직접 실행해서 출력
프레임, 모든 사용법 오류, Herdr에 닿지 못할 때의 실패 경로를 확인합니다.

## 릴리스

커밋 제목은 [Conventional Commits](https://www.conventionalcommits.org/)를 씁니다.
`CHANGELOG.md`와 모든 릴리스 본문은 [git-cliff](https://git-cliff.org)가 그 커밋
제목들로 생성하므로, 커밋 제목이 곧 릴리스 노트입니다. 다음 릴리스에 뭐가 들어갈지는
`just release-notes`로 미리 볼 수 있습니다.

`herdr/install.sh`는 `herdr-plugin.toml`의 `version`으로 만든 `v<version>` 태그
릴리스를 받아옵니다. 그래서 릴리스는 이 셋을 일치시키는 작업입니다.

1. `Cargo.toml`과 `herdr-plugin.toml`의 `version`을 같은 값으로 올립니다.
2. `just changelog`로 `CHANGELOG.md`를 다시 만들어 커밋합니다.
3. `just check-version v<version>`으로 확인한 뒤 태그를 푸시합니다.

워크플로가 모든 타깃을 빌드해서 `sha256` 사이드카와 서명된 빌드 프로버넌스를
함께 붙이고, 전부 올라간 다음에야 릴리스를 공개합니다. 아카이브 검증은
`gh attestation verify <archive> --repo huketo/herdr-sheep`.

## 라이선스

MIT. [LICENSE](LICENSE) 참고.
