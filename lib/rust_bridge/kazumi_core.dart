import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';

// FFI 函数类型定义
typedef RkVersionNative = Pointer<Utf8> Function();
typedef RkVersionDart = Pointer<Utf8> Function();

typedef RkFreeStringNative = Void Function(Pointer<Utf8>);
typedef RkFreeStringDart = void Function(Pointer<Utf8>);

typedef RkParseRuleNative = Pointer<Utf8> Function(Pointer<Utf8>);
typedef RkParseRuleDart = Pointer<Utf8> Function(Pointer<Utf8>);

typedef RkExtractBangumiNative = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>);
typedef RkExtractBangumiDart = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>);

typedef RkExtractVideoSourcesNative = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>);
typedef RkExtractVideoSourcesDart = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>);

typedef RkParseM3U8Native = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef RkParseM3U8Dart = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);

typedef RkSimilarityNative = Double Function(Pointer<Utf8>, Pointer<Utf8>);
typedef RkSimilarityDart = double Function(Pointer<Utf8>, Pointer<Utf8>);

typedef RkParseDanmakuNative = Pointer<Utf8> Function(Pointer<Utf8>);
typedef RkParseDanmakuDart = Pointer<Utf8> Function(Pointer<Utf8>);

typedef RkExtractXPathNative = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef RkExtractXPathDart = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);

typedef RkExtractXPathBatchNative = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef RkExtractXPathBatchDart = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);

typedef RkExtractSearchNative = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>);
typedef RkExtractSearchDart = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>);

/// rKazumi-core 引擎
class KazumiCore {
  static KazumiCore? _instance;
  late final DynamicLibrary _lib;
  bool _initialized = false;

  // FFI 函数
  late final RkVersionDart _version;
  late final RkFreeStringDart _freeString;
  late final RkParseRuleDart _parseRule;
  late final RkExtractBangumiDart _extractBangumi;
  late final RkExtractVideoSourcesDart _extractVideoSources;
  late final RkParseM3U8Dart _parseM3U8;
  late final RkSimilarityDart _similarity;
  late final RkParseDanmakuDart _parseDanmakuBilibili;
  late final RkExtractXPathDart _extractXPath;
  late final RkExtractXPathBatchDart _extractXPathBatch;
  late final RkExtractSearchDart _extractSearch;

  // LRU 缓存
  final _xpathCache = <String, Map<String, dynamic>>{};
  static const int _maxCacheSize = 128;

  /// 获取单例
  static KazumiCore get instance {
    _instance ??= KazumiCore._();
    return _instance!;
  }

  KazumiCore._();

  /// 初始化引擎，加载原生库
  bool initialize() {
    if (_initialized) return true;

    try {
      if (Platform.isAndroid) {
        _lib = DynamicLibrary.open('libkazumi_core.so');
      } else if (Platform.isWindows) {
        _lib = DynamicLibrary.open('kazumi_core.dll');
      } else if (Platform.isMacOS) {
        _lib = DynamicLibrary.open('libkazumi_core.dylib');
      } else if (Platform.isLinux) {
        _lib = DynamicLibrary.open('libkazumi_core.so');
      } else {
        throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
      }

      _version = _lib.lookupFunction<RkVersionNative, RkVersionDart>('rk_version');
      _freeString = _lib.lookupFunction<RkFreeStringNative, RkFreeStringDart>('rk_free_string');
      _parseRule = _lib.lookupFunction<RkParseRuleNative, RkParseRuleDart>('rk_parse_rule');
      _extractBangumi = _lib.lookupFunction<RkExtractBangumiNative, RkExtractBangumiDart>('rk_extract_bangumi');
      _extractVideoSources = _lib.lookupFunction<RkExtractVideoSourcesNative, RkExtractVideoSourcesDart>('rk_extract_video_sources');
      _parseM3U8 = _lib.lookupFunction<RkParseM3U8Native, RkParseM3U8Dart>('rk_parse_m3u8');
      _similarity = _lib.lookupFunction<RkSimilarityNative, RkSimilarityDart>('rk_similarity');
      _parseDanmakuBilibili = _lib.lookupFunction<RkParseDanmakuNative, RkParseDanmakuDart>('rk_parse_danmaku_bilibili');
      _extractXPath = _lib.lookupFunction<RkExtractXPathNative, RkExtractXPathDart>('rk_extract_xpath');
      _extractXPathBatch = _lib.lookupFunction<RkExtractXPathBatchNative, RkExtractXPathBatchDart>('rk_extract_xpath_batch');
      _extractSearch = _lib.lookupFunction<RkExtractSearchNative, RkExtractSearchDart>('rk_extract_search');

      _initialized = true;
      return true;
    } catch (e, st) {
      _initError = '$e\n$st';
      print('[KazumiCore] 加载原生库失败: $_initError');
      return false;
    }
  }

  /// 初始化失败时的错误信息
  String? _initError;
  String? get initError => _initError;

  /// 是否已初始化
  bool get isInitialized => _initialized;

  /// 获取引擎版本
  String get version {
    if (!_initialized) return 'unavailable';
    final ptr = _version();
    final result = ptr.toDartString();
    _freeString(ptr);
    return result;
  }

  /// 解析规则 JSON
  Map<String, dynamic>? parseRule(String json) {
    if (!_initialized) return null;
    final inputPtr = json.toNativeUtf8();
    final resultPtr = _parseRule(inputPtr);
    final resultStr = resultPtr.toDartString();
    _freeString(resultPtr);
    calloc.free(inputPtr);
    return jsonDecode(resultStr) as Map<String, dynamic>;
  }

  /// 提取番剧列表
  Map<String, dynamic>? extractBangumi(String html, String ruleName, String rulesJson) {
    if (!_initialized) return null;
    final htmlPtr = html.toNativeUtf8();
    final namePtr = ruleName.toNativeUtf8();
    final rulesPtr = rulesJson.toNativeUtf8();
    final resultPtr = _extractBangumi(htmlPtr, namePtr, rulesPtr);
    final resultStr = resultPtr.toDartString();
    _freeString(resultPtr);
    calloc.free(htmlPtr);
    calloc.free(namePtr);
    calloc.free(rulesPtr);
    return jsonDecode(resultStr) as Map<String, dynamic>;
  }

  /// 提取视频源
  Map<String, dynamic>? extractVideoSources(String html, String ruleName, String rulesJson) {
    if (!_initialized) return null;
    final htmlPtr = html.toNativeUtf8();
    final namePtr = ruleName.toNativeUtf8();
    final rulesPtr = rulesJson.toNativeUtf8();
    final resultPtr = _extractVideoSources(htmlPtr, namePtr, rulesPtr);
    final resultStr = resultPtr.toDartString();
    _freeString(resultPtr);
    calloc.free(htmlPtr);
    calloc.free(namePtr);
    calloc.free(rulesPtr);
    return jsonDecode(resultStr) as Map<String, dynamic>;
  }

  /// 解析 M3U8 播放列表
  Map<String, dynamic>? parseM3U8(String content, {String? baseUrl}) {
    if (!_initialized) return null;
    final contentPtr = content.toNativeUtf8();
    final urlPtr = (baseUrl ?? '').toNativeUtf8();
    final resultPtr = _parseM3U8(contentPtr, urlPtr);
    final resultStr = resultPtr.toDartString();
    _freeString(resultPtr);
    calloc.free(contentPtr);
    calloc.free(urlPtr);
    return jsonDecode(resultStr) as Map<String, dynamic>;
  }

  /// 计算字符串相似度
  double similarity(String a, String b) {
    if (!_initialized) return 0.0;
    final aPtr = a.toNativeUtf8();
    final bPtr = b.toNativeUtf8();
    final result = _similarity(aPtr, bPtr);
    calloc.free(aPtr);
    calloc.free(bPtr);
    return result;
  }

  /// 解析 Bilibili 弹幕 XML
  Map<String, dynamic>? parseDanmakuBilibili(String xml) {
    if (!_initialized) return null;
    final xmlPtr = xml.toNativeUtf8();
    final resultPtr = _parseDanmakuBilibili(xmlPtr);
    final resultStr = resultPtr.toDartString();
    _freeString(resultPtr);
    calloc.free(xmlPtr);
    return jsonDecode(resultStr) as Map<String, dynamic>;
  }

  /// 执行 XPath 查询（带 LRU 缓存）
  Map<String, dynamic>? extractXPath(String html, String xpath) {
    if (!_initialized) return null;
    final key = 'xpath:\$html:\$xpath';
    return _cached(key, () {
      final htmlPtr = html.toNativeUtf8();
      final xpathPtr = xpath.toNativeUtf8();
      final resultPtr = _extractXPath(htmlPtr, xpathPtr);
      final resultStr = resultPtr.toDartString();
      _freeString(resultPtr);
      calloc.free(htmlPtr);
      calloc.free(xpathPtr);
      return jsonDecode(resultStr) as Map<String, dynamic>;
    });
  }

  /// 批量执行 XPath 查询
  Map<String, dynamic>? extractXPathBatch(String html, Map<String, String> xpaths) {
    if (!_initialized) return null;
    final key = 'batch:\${html.length}:\${jsonEncode(xpaths)}';
    return _cached(key, () {
      final xpathsJson = jsonEncode(xpaths);
      final htmlPtr = html.toNativeUtf8();
      final xpathsPtr = xpathsJson.toNativeUtf8();
      final resultPtr = _extractXPathBatch(htmlPtr, xpathsPtr);
      final resultStr = resultPtr.toDartString();
      _freeString(resultPtr);
      calloc.free(htmlPtr);
      calloc.free(xpathsPtr);
      return jsonDecode(resultStr) as Map<String, dynamic>;
    });
  }

  /// 执行搜索提取（先查 list 节点，再在每个节点内相对查 name 和 url）
  Map<String, dynamic>? extractSearch(String html, String listXpath, String nameXpath, String urlXpath) {
    if (!_initialized) return null;
    final key = 'search:\${html.length}:\$listXpath:\$nameXpath:\$urlXpath';
    return _cached(key, () {
      final htmlPtr = html.toNativeUtf8();
      final listPtr = listXpath.toNativeUtf8();
      final namePtr = nameXpath.toNativeUtf8();
      final urlPtr = urlXpath.toNativeUtf8();
      final resultPtr = _extractSearch(htmlPtr, listPtr, namePtr, urlPtr);
      final resultStr = resultPtr.toDartString();
      _freeString(resultPtr);
      calloc.free(htmlPtr);
      calloc.free(listPtr);
      calloc.free(namePtr);
      calloc.free(urlPtr);
      return jsonDecode(resultStr) as Map<String, dynamic>;
    });
  }

  /// 带缓存的查询
  Map<String, dynamic>? _cached(String key, Map<String, dynamic> Function() query) {
    if (_xpathCache.containsKey(key)) {
      return _xpathCache[key];
    }
    if (_xpathCache.length >= _maxCacheSize) {
      _xpathCache.remove(_xpathCache.keys.first);
    }
    final result = query();
    _xpathCache[key] = result;
    return result;
  }

  /// 清空缓存
  void clearCache() {
    _xpathCache.clear();
  }

  /// 释放原生库资源
  void dispose() {
    _initialized = false;
  }
}