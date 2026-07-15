import 'package:html/dom.dart';
import 'package:html/parser.dart';
import 'package:kazumi/modules/roads/road_module.dart';
import 'package:kazumi/modules/search/plugin_search_module.dart';
import 'package:kazumi/plugins/anti_crawler_config.dart';
import 'package:kazumi/services/plugin/rule_engine_models.dart';
import 'package:kazumi/utils/episode_url.dart';
import 'package:xpath_selector_html_parser/xpath_selector_html_parser.dart';
import 'package:kazumi/rust_bridge/kazumi_core.dart';

/// XPath 规则策略
class XPathRuleStrategy {
  const XPathRuleStrategy();

  RuleSearchParseResult parseSearch(
    String raw,
    RuleExecutionConfig config,
  ) {
    final core = KazumiCore.instance;
    if (!core.isInitialized) {
      throw StateError('Rust 引擎未初始化，无法执行 XPath 查询');
    }

    // 第1步：获取 searchList 节点
    final listResult = core.extractXPath(raw, config.searchList);
    if (listResult == null || listResult['type'] != 'nodes') {
      throw XPathRuleFormatException(
        'Rust XPath 查询失败: \${listResult?['error'] ?? '未知错误'}',
        kind: XPathRuleFormatKind.invalidSelector,
      );
    }

    final nodes = listResult['nodes'] as List<dynamic>? ?? [];
    final items = <SearchItem>[];
    final fragments = <String>[];
    final diagnostics = <String>[];

    for (var index = 0; index < nodes.length; index++) {
      try {
        final node = nodes[index] as Map<String, dynamic>;
        final nodeHtml = node['outer_html'] as String? ?? node['html'] as String? ?? '';
        if (nodeHtml.isEmpty) {
          diagnostics.add('搜索节点 \$index 没有 HTML 内容，已跳过');
          continue;
        }

        // 在节点上下文中求值 searchName
        final nameResult = core.extractXPath(nodeHtml, config.searchName);
        String name = '';
        if (nameResult != null && nameResult['type'] == 'string') {
          name = nameResult['value'] as String? ?? '';
        } else if (nameResult != null && nameResult['type'] == 'nodes') {
          final nameNodes = nameResult['nodes'] as List<dynamic>? ?? [];
          if (nameNodes.isNotEmpty) {
            name = (nameNodes[0] as Map<String, dynamic>)['text'] as String? ?? '';
          }
        }

        // 在节点上下文中求值 searchResult
        final srcResult = core.extractXPath(nodeHtml, config.searchResult);
        String source = '';
        if (srcResult != null && srcResult['type'] == 'string') {
          source = srcResult['value'] as String? ?? '';
        } else if (srcResult != null && srcResult['type'] == 'nodes') {
          final srcNodes = srcResult['nodes'] as List<dynamic>? ?? [];
          if (srcNodes.isNotEmpty) {
            final attrs = (srcNodes[0] as Map<String, dynamic>)['attributes']
                as Map<String, dynamic>? ?? {};
            source = attrs['href'] as String? ?? '';
          }
        }

        if (name.isEmpty || source.isEmpty) {
          diagnostics.add('搜索节点 \$index 缺少名称或来源，已跳过');
          continue;
        }
        items.add(SearchItem(name: name, src: source));
        fragments.add(nodeHtml);
      } catch (e) {
        diagnostics.add('搜索节点 \$index 解析失败: \$e');
      }
    }
    return RuleSearchParseResult(
      items: items,
      matchedFragments: fragments,
      diagnostics: diagnostics,
    );
  }

  RuleChapterParseResult parseChapters(
    String raw,
    RuleExecutionConfig config,
  ) {
    final core = KazumiCore.instance;
    if (!core.isInitialized) {
      throw StateError('Rust 引擎未初始化，无法执行 XPath 查询');
    }

    // 1. 获取所有播放线路节点 (使用 Rust)
    if (config.chapterRoads.isEmpty) {
      throw XPathRuleFormatException(
        '章节线路 XPath 为空',
        kind: XPathRuleFormatKind.invalidSelector,
      );
    }
    final roadsResult = core.extractXPath(raw, config.chapterRoads);
    if (roadsResult == null || (roadsResult['type'] != 'nodes' && roadsResult['type'] != 'string')) {
      throw XPathRuleFormatException(
        'Rust XPath 查询失败: \${roadsResult?['error'] ?? '未知错误'}',
        kind: XPathRuleFormatKind.invalidSelector,
      );
    }

    final roadNodes = roadsResult['nodes'] as List<dynamic>? ?? [];
    final roads = <Road>[];
    final diagnostics = <String>[];

    for (var roadIndex = 0; roadIndex < roadNodes.length; roadIndex++) {
      try {
        final roadNode = roadNodes[roadIndex] as Map<String, dynamic>;
        final roadHtml = roadNode['outer_html'] as String? ?? '';

        // 2. 在每个线路节点中获取剧集列表
        final epsResult = core.extractXPath(roadHtml, config.chapterResult);
        if (epsResult == null || epsResult['type'] != 'nodes') {
          diagnostics.add('线路 \$roadIndex 没有有效剧集，已跳过');
          continue;
        }

        final episodeNodes = epsResult['nodes'] as List<dynamic>? ?? [];
        final urls = <String>[];
        final names = <String>[];

        for (var episodeIndex = 0;
            episodeIndex < episodeNodes.length;
            episodeIndex++) {
          try {
            final epNode = episodeNodes[episodeIndex] as Map<String, dynamic>;
            final attrs = epNode['attributes'] as Map<String, dynamic>? ?? {};
            final source = attrs['href'] as String? ?? '';
            if (source.isEmpty) {
              diagnostics.add(
                '线路 \$roadIndex 的剧集节点 \$episodeIndex 缺少 URL，已跳过',
              );
              continue;
            }
            final name = (epNode['text'] as String? ?? '')
                .replaceAll(RegExp(r'\s+'), '');
            urls.add(normalizeEpisodeUrl(config.baseUrl, source));
            names.add(name.isEmpty ? '第\${episodeIndex + 1}集' : name);
          } catch (error) {
            diagnostics.add(
              '线路 \$roadIndex 的剧集节点 \$episodeIndex 解析失败: \$error',
            );
          }
        }

        if (urls.isEmpty) {
          diagnostics.add('线路 \$roadIndex 没有有效剧集，已跳过');
          continue;
        }
        roads.add(
          Road(
            name: '播放线路\${roads.length + 1}',
            data: urls,
            identifier: names,
          ),
        );
      } catch (error) {
        diagnostics.add('线路节点 \$roadIndex 解析失败: \$error');
      }
    }
    return RuleChapterParseResult(
      roads: roads,
      diagnostics: diagnostics,
    );
  }

  // 兼容旧版 Dart 实现的辅助方法
  bool detectsCaptchaChallenge(
    String raw,
    AntiCrawlerConfig config, {
    Element? htmlElement,
  }) {
    if (!config.enabled) return false;
    final detectValue = config.captchaDetectValue.trim();
    if (detectValue.isNotEmpty) {
      switch (config.captchaDetectType) {
        case CaptchaDetectType.text:
          return raw.contains(detectValue);
        case CaptchaDetectType.regex:
          try {
            return RegExp(
              detectValue,
              caseSensitive: false,
              dotAll: true,
            ).hasMatch(raw);
          } on FormatException {
            return false;
          }
        case CaptchaDetectType.xpath:
        default:
          final root = htmlElement ?? _documentElement(raw);
          return _runSelector(
                XPathRuleField.captchaDetectValue,
                detectValue,
                () => root.queryXPath(detectValue).node,
              ) !=
              null;
      }
    }

    final root = htmlElement ?? _documentElement(raw);
    final fallbackSelectors = <(XPathRuleField, String)>[
      (XPathRuleField.captchaImage, config.captchaImage),
      (XPathRuleField.captchaButton, config.captchaButton),
    ];
    for (final (field, expression) in fallbackSelectors) {
      if (expression.trim().isEmpty) continue;
      final node = _runSelector(
        field,
        expression,
        () => root.queryXPath(expression).node,
      );
      if (node != null) return true;
    }
    return false;
  }

  Element _documentElement(String raw) {
    try {
      final element = parse(raw).documentElement;
      if (element == null) {
        throw const XPathRuleFormatException(
          'HTML 响应没有根节点',
          kind: XPathRuleFormatKind.invalidDocument,
        );
      }
      return element;
    } on XPathRuleFormatException {
      rethrow;
    } catch (error) {
      throw XPathRuleFormatException(
        'HTML 响应解析失败',
        kind: XPathRuleFormatKind.invalidDocument,
        cause: error,
      );
    }
  }

  T _runSelector<T>(
    XPathRuleField field,
    String expression,
    T Function() query,
  ) {
    final label = _fieldLabel(field);
    if (expression.trim().isEmpty) {
      throw XPathRuleFormatException(
        '\$label XPath 不能为空',
        kind: XPathRuleFormatKind.invalidSelector,
        field: field,
        expression: expression,
      );
    }
    try {
      return query();
    } on XPathRuleFormatException {
      rethrow;
    } catch (error) {
      throw XPathRuleFormatException(
        '\$label XPath 查询失败',
        kind: XPathRuleFormatKind.invalidSelector,
        field: field,
        expression: expression,
        cause: error,
      );
    }
  }

  String _fieldLabel(XPathRuleField field) {
    switch (field) {
      case XPathRuleField.searchList:
        return '搜索列表';
      case XPathRuleField.searchName:
        return '搜索名称';
      case XPathRuleField.searchResult:
        return '搜索结果';
      case XPathRuleField.chapterRoads:
        return '章节线路';
      case XPathRuleField.chapterResult:
        return '章节结果';
      case XPathRuleField.captchaDetectValue:
        return '验证码检测';
      case XPathRuleField.captchaImage:
        return '验证码图片';
      case XPathRuleField.captchaButton:
        return '验证码按钮';
    }
  }

  String _fragment(Node node) {
    return node is Element ? node.outerHtml : node.text ?? '';
  }
}