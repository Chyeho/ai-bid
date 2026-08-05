from __future__ import annotations

import csv
import hashlib
import io
import json
import re
import sys
import time
import urllib.request
from pathlib import Path
from urllib.parse import urlparse

from pypdf import PdfReader, PdfWriter
from reportlab.lib.pagesizes import A4
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen import canvas


ROOT = Path(__file__).resolve().parent
SOURCE_DIR = ROOT / "sources"
MUTATED_DIR = ROOT / "mutated"
DATA_DIR = ROOT / "data"
FONT_PATH = Path(r"C:\Windows\Fonts\simhei.ttf")

SOURCES = [
    ("BID-001", "2025信息化城市管理运行项目", "北京市城市管理委员会", 2025, "信息化", "https://csglw.beijing.gov.cn/zwxx/zwtzgg/202506/P020250626595469092921.pdf"),
    ("BID-002", "数据中心基础设施升级采购项目", "中国人民银行征信中心", 2025, "信息化", "https://jzcg.pbc.gov.cn/gpx-bid-file/ZZZZZZ/gpx-template/2025/6/9/2c96d1ac96ce7c71019753a7564d15b3.pdf?accessCode=a6223f36b722e245898cbf758adeadb6"),
    ("BID-003", "信息化建设软硬件采购项目", "陕西省政府采购项目", 2025, "信息化", "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/ZF_JGBM_000003/zone/2025/1/12/project/gpx-template/8a69c19196ce13280196f6d632064292.pdf?accessCode=1a4049b8286ea094eed651f9d4178c36"),
    ("BID-004", "陕西省公开招标采购项目A", "陕西省政府采购项目", 2025, "综合", "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/ZF_JGBM_000003/zone/2025/1/12/project/gpx-template/8a69c21f9a63addd019a9636b8213237.pdf?accessCode=301f123b4b21629a24bfa0df18a6098a"),
    ("BID-005", "2025国产试剂耗材采购（三次）", "陕西省政府采购项目", 2025, "医疗耗材", "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/ZF_JGBM_000003/zone/2025/1/12/project/gpx-template/8a69c4e298b3927e0198dfe312b21a6a.pdf?accessCode=38345b044cc6fcb365c8981774200232"),
    ("BID-006", "YLZC2026-G3-810023-GXYJ采购项目", "广西政府采购项目", 2026, "服务", "https://obs.gcy.zfcg.gxzf.gov.cn/guangxi-gov-open-doc/1014AN/450981/10010898738/20263/c02edc8a-d6a8-4bd0-89e3-bb57f083b9c2.pdf"),
    ("BID-007", "医疗设备放射类及临床辅助影像系统项目", "北京经济技术开发区政府采购项目", 2025, "医疗设备", "https://kfqgw.beijing.gov.cn/zwgkkfq/zdlyxxgk/zfcg/202508/P020250825558585189605.pdf"),
    ("BID-008", "上海市黄浦区物业管理项目A", "上海市黄浦区政府采购项目", 2025, "物业", "https://jpg.zfcg.sh.gov.cn/sh-gov-open-doc/1108QZ/58df2aea-5e99-4c27-9110-44ed36c08549.pdf"),
    ("BID-009", "黄姚古镇医院医疗设备采购", "广西政府采购项目", 2025, "医疗设备", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1024FPA/459900/10013281003/20256/34a158e0-5404-4b08-bc2e-fd058461d65c.pdf"),
    ("BID-010", "QZZC2025-G1-020081-BJCJ医疗设备项目", "广西政府采购项目", 2025, "医疗设备", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1023FP/450199/10008173695/20258/17534929-bbb6-4800-a73d-5985237d5c91.pdf"),
    ("BID-011", "税务物业服务及保安项目", "国家税务总局深圳市税务局", 2021, "物业", "https://shenzhen.chinatax.gov.cn/sztax/xxgk/zfcg/jggg/202104/ac97c4a4cbf14afaaed872275e0e17db/files/38d4b7d793dd4eca94ee5fc78bdd6735.pdf"),
    ("BID-012", "崇左学校保安服务项目", "广西政府采购项目", 2024, "安保", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1024FPA/undefined/null/20247/246e3700-5c0b-45ef-bddb-338089d19551.pdf"),
    ("BID-013", "百色学校设备采购项目", "广西政府采购项目", 2024, "教育设备", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1023FP/undefined/null/20247/26ea80d5-c0f5-48be-b842-9909ce924257.pdf"),
    ("BID-014", "宁明学校保安服务项目", "广西政府采购项目", 2025, "安保", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1024FPA/450107/10009312885/20256/92f65423-6ef6-47e9-9fa1-3694d57f69dc.pdf"),
    ("BID-015", "上海市黄浦区物业管理项目B", "上海市黄浦区政府采购项目", 2025, "物业", "https://jpg.zfcg.sh.gov.cn/sh-gov-open-doc/1108QZ/7517ab89-c345-43e6-8ad2-2cc45bda1796.pdf"),
    ("BID-016", "上海城市综合管理服务项目", "上海市黄浦区政府采购项目", 2025, "城市管理", "https://jpg.zfcg.sh.gov.cn/sh-gov-open-doc/1108QZ/6e6c6360-b134-402a-8054-961b5ed0c18e.pdf"),
    ("BID-017", "上海城运AI信息系统项目", "上海市黄浦区政府采购项目", 2026, "信息化", "https://jpg.zfcg.sh.gov.cn/sh-gov-open-doc/1108QZ/1d182af9-c1bc-47fc-8ba8-365f2f3f7d25.pdf"),
    ("BID-018", "黄浦社区卫生物业管理项目", "上海市黄浦区政府采购项目", 2025, "物业", "https://jpg.zfcg.sh.gov.cn/sh-gov-open-doc/1108QZ/a379b261-cb33-4485-ad19-5347327fad4c.pdf"),
    ("BID-019", "黄浦教育城域网无线租赁项目", "上海市黄浦区政府采购项目", 2025, "信息化", "https://jpg.zfcg.sh.gov.cn/sh-gov-open-doc/1108QZ/60e25a14-ae1b-4ceb-993e-33c6b06f6637.pdf"),
    ("BID-020", "上海市黄浦区物业管理项目C", "上海市黄浦区政府采购项目", 2025, "物业", "https://jpg.zfcg.sh.gov.cn/sh-gov-open-doc/1108QZ/32a671bb-8d51-440a-844e-ad93d66998d4.pdf"),
    ("BID-021", "医疗设备资产整体管理服务项目", "广西政府采购项目", 2025, "医疗服务", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1014AN/450103/10007339432/202511/2c546c9e-bc20-4b09-a120-4f1ffee65640.pdf"),
    ("BID-022", "北海妇幼医疗设备采购项目", "广西政府采购项目", 2024, "医疗设备", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1024FPA/undefined/null/20247/aadc452a-3fd8-4e1a-b59e-d4b7544ad057.pdf"),
    ("BID-023", "江州学校食材配送项目", "广西政府采购项目", 2024, "食材配送", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1014AN/undefined/450103/10008148327/202412/afe9962e-b550-4c7a-9c6b-9c80295f04e5.pdf"),
    ("BID-024", "灵山中医院医疗设备项目", "广西政府采购项目", 2025, "医疗设备", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1024FPA/undefined/459900/10010498870/20254/20009004-b9a6-4cc4-90b2-44f6cfae88c8.pdf"),
    ("BID-025", "广西医疗设备采购项目A", "广西政府采购项目", 2024, "医疗设备", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1024FPA/undefined/450103/10007876793/202411/6f8ac23c-b369-4a08-8638-990cefe99f87.pdf"),
    ("BID-026", "2023年度第一批医疗设备采购", "广西政府采购项目", 2024, "医疗设备", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1014AN/450103/1000659093/20244/e8455595-5988-4fea-851c-718d5fbebcc8.pdf"),
    ("BID-027", "北京市园林绿化局公开招标项目A", "北京市园林绿化局", 2026, "服务", "https://yllhj.beijing.gov.cn/zwgk/zfcg/cgjg/202601/P020260120532658683057.pdf"),
    ("BID-028", "北京市园林绿化局公开招标项目B", "北京市园林绿化局", 2026, "服务", "https://yllhj.beijing.gov.cn/zwgk/zfcg/cgjg/202601/P020260120527660206221.pdf"),
    ("BID-029", "2024—2026城市管理服务项目", "北京市城市管理委员会", 2023, "城市管理", "https://csglw.beijing.gov.cn/zwxx/zwtzgg/202312/P020231222556396568588.pdf"),
    ("BID-030", "北京经开区政府采购项目", "北京经济技术开发区政府采购项目", 2022, "综合", "https://kfqgw.beijing.gov.cn/zwgkkfq/zdlyxxgk/zfcg/202211/P020221122537517021297.pdf"),
    ("BID-031", "北京市园林绿化局公开招标项目C", "北京市园林绿化局", 2026, "服务", "https://yllhj.beijing.gov.cn/zwgk/zfcg/cgjg/202602/P020260206385158650584.pdf"),
    ("BID-032", "综合服务能力提升医疗设备采购", "陕西省政府采购项目", 2022, "医疗设备", "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/610324/gpx-template/2022/11/15/8a69c0398405ac3201847955845a0266.pdf"),
    ("BID-033", "西安保障房物业服务项目", "陕西省政府采购项目", 2024, "物业", "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/ZF_JGBM_000003/610116/2024/10/24/8a69c6f491b2400b0192b355852f6d8f/gpx-template/8a69c25092d1e294019347494cf6132d.pdf?accessCode=2128d20f3aa32e60cc3b95fda1e61883"),
    ("BID-034", "卫健系统信息化硬件采购项目", "陕西省政府采购项目", 2025, "信息化", "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/ZF_JGBM_000003/zone/2025/1/12/project/gpx-template/8a69c77594582cc70194f2bacf4e13b4.pdf?accessCode=67c921e671d920889998805996f1d55f"),
    ("BID-035", "西林幼儿园设备采购项目", "广西政府采购项目", 2024, "教育设备", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1024FPA/451002/10006383011/20245/d556c536-93aa-4af6-a07a-8b19dce65872.pdf"),
    ("BID-036", "柳州警务云运维服务项目", "广西政府采购项目", 2025, "信息化运维", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1014AN/450103/10009994593/202510/bc2bd226-38f4-4d66-b5fb-391b86242343.pdf"),
    ("BID-037", "钦州市第二人民医院设备采购", "广西政府采购项目", 2025, "医疗设备", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1024FPA/450199/10008182182/20255/a8a421be-0df3-4a35-b82c-8689e93605b1.pdf"),
    ("BID-038", "桂平中医院设备采购项目", "广西政府采购项目", 2025, "医疗设备", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1023FP/undefined/450881/10009458513/20252/c4631fcf-426c-447d-8a7d-19536c3e61ae.pdf"),
    ("BID-039", "柳州职业大学新能源汽车实训项目", "广西政府采购项目", 2025, "教育设备", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1024FPA/undefined/450103/10007338844/20251/6a7c0c48-a97d-4dab-b57c-0c27072490fc.pdf"),
    ("BID-040", "钦南妇幼医疗设备采购项目", "广西政府采购项目", 2025, "医疗设备", "https://oss-gxhlw.unicloudgov.com/guangxi-gov-open-doc/1024FPA/undefined/459900/10007340114/20251/5de9da31-4422-473d-8d65-8ea12c8a15c8.pdf"),
    ("BID-041", "上海市黄浦区物业管理项目D", "上海市黄浦区政府采购项目", 2026, "物业", "https://jpg.zfcg.sh.gov.cn/sh-gov-open-doc/1108QZ/eec7b7b6-2c5b-467a-a8d0-9d5295604e9d.pdf"),
    ("BID-042", "2026网络产品包3采购项目", "中国人民银行集中采购中心", 2026, "信息化", "https://jzcg.pbc.gov.cn/gpx-bid-file/ZZZZZZ/gpx-template/2026/5/7/e5abc09ccf584481a4a8fb0bd8397ae5.pdf?accessCode=617e1f84e544c72de09f31a1578d98ce"),
    ("BID-043", "RH-WTGK2025039采购项目", "中国人民银行集中采购中心", 2025, "综合", "https://jzcg.pbc.gov.cn/gpx-bid-file/ZZZZZZ/gpx-template/2025/9/11/2c96d1ac991479b3019937d59cbf1667.pdf?accessCode=3ec8e835ae8b5effa6c22a4063fb45e1"),
    ("BID-044", "RH-WTGK2024075采购项目", "中国人民银行集中采购中心", 2025, "综合", "https://jzcg.pbc.gov.cn/gpx-bid-file/ZZZZZZ/gpx-template/2025/1/21/2c96d1ac9426abce0194880ec8d605e3.pdf?accessCode=241585603a37c2313b2cd00850ab5a69"),
    ("BID-045", "RH-WTGK2025038采购项目", "中国人民银行集中采购中心", 2025, "综合", "https://jzcg.pbc.gov.cn/gpx-bid-file/ZZZZZZ/gpx-template/2025/8/28/2c96d1ac986566d80198efd1dc087c1c.pdf?accessCode=a6206838947a5a39932801b68772efac"),
    ("BID-046", "北金所云平台采购项目", "中国人民银行集中采购中心", 2025, "信息化", "https://jzcg.pbc.gov.cn/gpx-bid-file/ZZZZZZ/gpx-template/2025/1/7/2c96d1ac9426abce01943f597e1f340f.pdf?accessCode=4456f1d7fc6273f93958e7d2e62f72c8"),
    ("BID-047", "人民银行1号楼厨房采购项目", "中国人民银行集中采购中心", 2025, "餐饮设备", "https://jzcg.pbc.gov.cn/gpx-bid-file/ZZZZZZ/gpx-template/2025/2/10/2c96d1ac9426abce0194eec868ae435f.pdf?accessCode=751210a670a059221723f8c393373a81"),
    ("BID-048", "RH-GK2024008采购项目", "中国人民银行集中采购中心", 2024, "综合", "https://jzcg.pbc.gov.cn/gpx-bid-file/ZZZZZZ/gpx-template/2024/10/25/2c96d1ad9275e8220192c29d582670ef.pdf?accessCode=53fb34bc99bedbe0b79c02747d9cec9e"),
    ("BID-049", "RH-WTGK2024028采购项目", "中国人民银行集中采购中心", 2024, "综合", "https://jzcg.pbc.gov.cn/gpx-bid-file/ZZZZZZ/gpx-template/2024/8/9/2c96d1ac90ef34230191361cd5f0519e.pdf?accessCode=0f3725ddd1b40edf9bca107096960cd1"),
    ("BID-050", "RH-WTGK2025018采购项目", "中国人民银行集中采购中心", 2025, "综合", "https://jzcg.pbc.gov.cn/gpx-bid-file/ZZZZZZ/gpx-template/2025/7/15/2c96d1ad96ce7c6701980d2fa8d818d4.pdf?accessCode=e766177c9e661cfb4713818711f3b5ef"),
]

LEGAL = {
    "gov_law_22": {
        "title": "《中华人民共和国政府采购法》第二十二条第二款",
        "url": "https://www.samr.gov.cn/zw/zfxxgk/fdzdgknr/bgt/art/2023/art_47b5807c40c040368eb5f13b489d6c43.html",
    },
    "impl_20": {
        "title": "《中华人民共和国政府采购法实施条例》第二十条",
        "url": "https://jdjc.mof.gov.cn/fgzd/202202/t20220225_3790482.htm",
    },
    "order87_17": {
        "title": "《政府采购货物和服务招标投标管理办法》（财政部令第87号）第十七条",
        "url": "https://www.mof.gov.cn/gp/xxgkml/tfs/201707/t20170718_2652766.htm",
    },
    "order87_33": {
        "title": "《政府采购货物和服务招标投标管理办法》（财政部令第87号）第三十三条",
        "url": "https://www.mof.gov.cn/gp/xxgkml/tfs/201707/t20170718_2652766.htm",
    },
    "order87_55": {
        "title": "《政府采购货物和服务招标投标管理办法》（财政部令第87号）第五十五条",
        "url": "https://www.mof.gov.cn/gp/xxgkml/tfs/201707/t20170718_2652766.htm",
    },
    "gov_law_35": {
        "title": "《中华人民共和国政府采购法》第三十五条",
        "url": "https://www.samr.gov.cn/zw/zfxxgk/fdzdgknr/bgt/art/2023/art_47b5807c40c040368eb5f13b489d6c43.html",
    },
    "demand_9": {
        "title": "《政府采购需求管理办法》（财库〔2021〕22号）第九条",
        "url": "https://www.mof.gov.cn/gkml/caizhengwengao/wg2021/wg202005/202109/t20210917_3753625.htm",
    },
}

TAXONOMY = [
    {
        "code": "C01_LOCAL_REGISTRATION",
        "name": "地域注册限制",
        "severity": "high",
        "critical": True,
        "aliases": ["地域限制", "地域歧视", "本地注册", "本地企业限制", "差别待遇", "歧视待遇"],
        "reason": "将注册地、分支机构或本地经营场所作为准入条件，排斥外地供应商。",
        "suggestion": "删除注册地和本地机构要求，改为与履约能力直接相关且全国统一适用的条件。",
        "legal": ["gov_law_22", "impl_20"],
        "variants": [
            "投标人须在采购人所在地注册，且已连续经营满三年；外地企业不接受投标。",
            "仅接受本市行政区域内注册的供应商参加，异地注册企业视为资格不合格。",
            "供应商必须在项目所在地设有注册分公司满两年，否则投标无效。",
        ],
    },
    {
        "code": "C02_BRAND_LOCK",
        "name": "指定品牌且不接受同等产品",
        "severity": "high",
        "critical": True,
        "aliases": ["指定品牌", "品牌指定", "品牌锁定", "商标限定", "排斥同等产品", "特定产品"],
        "reason": "直接指定唯一品牌并排除同等产品，可能指向特定供应商或产品。",
        "suggestion": "改为可验证的功能、性能和质量指标；确需引用品牌时增加“或相当于”并给出等同性标准。",
        "legal": ["impl_20", "demand_9"],
        "variants": [
            "核心设备只能采用华为品牌，任何其他品牌或同等性能产品均不接受。",
            "本项目指定使用海康威视产品，不接受替代品牌、兼容产品或同等档次产品。",
            "投标产品必须为联想原厂指定型号，参数更优的其他品牌亦作无效投标处理。",
        ],
    },
    {
        "code": "C03_UNRELATED_CERT",
        "name": "设置与履约无关的资格条件",
        "severity": "high",
        "critical": True,
        "aliases": ["资格条件不相关", "不合理资质", "资质排他", "认证限制", "无关证书", "差别待遇"],
        "reason": "将与采购标的履约能力无直接关系的认证、荣誉作为资格门槛。",
        "suggestion": "删除无关认证，仅保留法律规定或与项目履约直接相关且必要的资格条件。",
        "legal": ["gov_law_22", "impl_20"],
        "variants": [
            "供应商须同时具有食品安全管理体系认证和五星级售后服务认证，否则不得投标；上述证书与本项目履约内容无关。",
            "投标人必须获得省级以上“诚信示范企业”荣誉并连续保持五年，未获得者资格审查不通过。",
            "供应商须具有与本采购标的无关的环境服务认证证书，且认证时间不少于三年。",
        ],
    },
    {
        "code": "C04_REGIONAL_PERFORMANCE",
        "name": "特定区域业绩限制",
        "severity": "high",
        "critical": True,
        "aliases": ["本地业绩", "区域业绩", "地域歧视", "特定行政区域业绩", "业绩限制", "差别待遇"],
        "reason": "以特定行政区域内的业绩作为资格或加分条件，对其他供应商形成不合理排斥。",
        "suggestion": "将业绩要求改为与项目规模和复杂度相当的全国范围同类业绩，不限定行政区域。",
        "legal": ["impl_20"],
        "variants": [
            "投标人近三年必须具有本市政府采购项目业绩不少于5项，外省市业绩不予认可。",
            "仅对本省行政区域内完成的同类合同计分，每项2分，其他地区业绩不得分。",
            "供应商须提供采购人所在区县的同类服务案例，跨区域案例不作为有效业绩。",
        ],
    },
    {
        "code": "C05_SCALE_THRESHOLD",
        "name": "以经营规模设置资格门槛",
        "severity": "high",
        "critical": True,
        "aliases": ["注册资本限制", "营业收入限制", "资产规模限制", "规模条件", "规模门槛", "资质排他", "资格门槛"],
        "reason": "把注册资本、营业收入或资产总额等规模条件作为资格要求。",
        "suggestion": "删除规模指标，改为信用、人员、技术方案和可核验履约能力要求。",
        "legal": ["order87_17", "gov_law_22"],
        "variants": [
            "投标人注册资本不得低于5000万元，上一年度营业收入不得低于1亿元。",
            "供应商资产总额须达到8000万元以上，否则资格审查不通过。",
            "投标人近两年平均营业收入必须超过本项目预算金额的十倍。",
        ],
    },
    {
        "code": "H01_SHORT_DEADLINE",
        "name": "投标准备期不足",
        "severity": "high",
        "critical": False,
        "aliases": ["投标期限不足", "不足20日", "时间过短", "法定期限", "开标期限"],
        "reason": "公开招标从招标文件发出至投标截止少于法定最短期限。",
        "suggestion": "将投标截止时间顺延，确保自招标文件发出之日起不少于20日。",
        "legal": ["gov_law_35"],
        "variants": [
            "本附加条款发布之日起第5日即为投标截止日和开标日，逾期不予受理。",
            "招标文件发出后7日内完成投标，次日组织开标。",
            "供应商须在获取本条款后10日内递交投标文件，该期限不作顺延。",
        ],
    },
    {
        "code": "H02_EXCESSIVE_DEPOSIT",
        "name": "投标保证金比例过高",
        "severity": "high",
        "critical": False,
        "aliases": ["保证金超限", "投标保证金", "超过2%", "保证金比例", "资金门槛"],
        "reason": "要求的投标保证金超过采购项目预算金额的2%。",
        "suggestion": "将投标保证金降至项目预算金额2%以内，并允许依法合规的非现金形式。",
        "legal": ["order87_33"],
        "variants": [
            "投标保证金按项目预算金额的8%收取，未足额缴纳的投标无效。",
            "供应商须缴纳相当于采购预算5%的投标保证金，仅接受银行转账。",
            "本项目投标保证金为预算金额的10%，到账后方可参与评审。",
        ],
    },
    {
        "code": "H03_OEM_AUTHORIZATION",
        "name": "将厂家授权作为资格条件",
        "severity": "high",
        "critical": False,
        "aliases": ["厂家授权", "原厂授权", "制造商授权", "授权函资格", "资格限制"],
        "reason": "对非进口货物，将生产厂家授权、承诺或背书作为资格要求。",
        "suggestion": "删除资格阶段的厂家授权门槛；如确有售后需要，将其转化为中标后可验证的履约要求。",
        "legal": ["order87_17", "impl_20"],
        "variants": [
            "投标人必须在资格文件中提交生产厂家针对本项目出具的唯一授权函，否则投标无效。",
            "非制造商投标的，须提供原厂授权和原厂售后承诺，两者均作为资格条件。",
            "代理商必须取得设备生产企业逐页盖章的项目授权书，缺少即否决投标。",
        ],
    },
    {
        "code": "H04_SUBJECTIVE_SCORING",
        "name": "主观评分未细化量化",
        "severity": "high",
        "critical": False,
        "aliases": ["评分未量化", "主观评分", "自由裁量", "评分标准不明确", "方案优良"],
        "reason": "评分标准仅使用优、良、一般等描述，没有可核验的量化分档。",
        "suggestion": "将评分因素细化为客观、可验证的指标，明确每档条件和对应分值。",
        "legal": ["order87_55", "demand_9"],
        "variants": [
            "实施方案由评委根据整体印象评分：优秀得10分，良好得6分，一般得2分，未规定各档客观判断标准。",
            "根据投标文件美观程度和评委主观感受酌情给0—12分，不设量化分档。",
            "服务方案先进、合理的得满分，较合理的酌情扣分，具体扣分由评委自行掌握。",
        ],
    },
    {
        "code": "H05_LOCAL_AWARD",
        "name": "本地奖项加分",
        "severity": "high",
        "critical": False,
        "aliases": ["本地奖项", "区域奖励", "地域歧视", "荣誉加分", "特定行政区域奖励", "差别待遇"],
        "reason": "将特定行政区域颁发的荣誉或奖项作为加分条件。",
        "suggestion": "删除地域性奖项加分，改为与履约质量直接相关的可验证指标。",
        "legal": ["impl_20"],
        "variants": [
            "获得本市行政机关颁发的优秀企业称号，每项加3分；其他地区同类荣誉不加分。",
            "近三年取得本省行业主管部门荣誉的得5分，省外奖项不予认可。",
            "具有采购人所在区县颁发的诚信企业证书得4分，其他证书不得分。",
        ],
    },
    {
        "code": "M01_VAGUE_ACCEPTANCE",
        "name": "验收标准模糊",
        "severity": "medium",
        "critical": False,
        "aliases": ["验收标准不明确", "验收模糊", "满意为准", "主观验收", "需求不清"],
        "reason": "验收条件仅以采购人满意或认可为准，缺少客观指标、程序和证据要求。",
        "suggestion": "明确验收主体、时间、测试方法、量化指标、记录和不合格处置机制。",
        "legal": ["demand_9"],
        "variants": [
            "项目完成后以采购人主观满意为唯一验收标准，采购人无需说明不通过理由。",
            "最终成果达到采购人认为合适的程度即视为验收，具体标准合同签订后另行口头通知。",
            "验收结果完全由采购人现场人员自行判断，不设置测试指标、验收记录和复核程序。",
        ],
    },
    {
        "code": "M02_UNBOUNDED_IP",
        "name": "知识产权责任无限扩大",
        "severity": "medium",
        "critical": False,
        "aliases": ["知识产权无限责任", "背景知识产权", "成果权属", "第三方索赔", "责任失衡"],
        "reason": "无边界转让供应商既有知识产权，并要求承担不受控制的无限责任。",
        "suggestion": "区分背景知识产权和项目成果，限定许可范围、责任条件与赔偿上限。",
        "legal": ["demand_9"],
        "variants": [
            "供应商须无偿转让其在本项目实施前已经拥有的全部知识产权，并对任何第三方主张承担无限责任。",
            "合同签订后供应商所有既有软件、工具和通用组件的权利均永久归采购人，且不另行计费。",
            "无论侵权原因是否由采购人指定方案造成，供应商均承担全部且无上限的知识产权赔偿责任。",
        ],
    },
    {
        "code": "M03_UNILATERAL_CHANGE",
        "name": "采购人可单方无限变更需求",
        "severity": "medium",
        "critical": False,
        "aliases": ["单方变更", "无限变更", "需求范围不清", "合同边界", "无偿增加"],
        "reason": "采购人可无偿、无限次增加工作范围，合同标的和价格边界不清。",
        "suggestion": "建立书面变更流程，明确范围、工期、费用调整和双方确认机制。",
        "legal": ["demand_9"],
        "variants": [
            "采购人可在履约期间无限次增加工作内容，供应商必须无偿完成且不得调整工期。",
            "采购人有权单方变更全部技术和服务范围，中标价格与交付日期均保持不变。",
            "任何新增需求均视为原合同范围，供应商不得提出费用或工期调整。",
        ],
    },
    {
        "code": "M04_CONFLICTING_DATES",
        "name": "关键日期相互矛盾",
        "severity": "medium",
        "critical": False,
        "aliases": ["日期冲突", "时间矛盾", "前后不一致", "截止时间冲突", "需求不清"],
        "reason": "同一条款对关键截止时间给出互相冲突的规定，可能导致响应和评审争议。",
        "suggestion": "统一公告、投标人须知、时间表和系统中的日期，并明确以哪一处为准。",
        "legal": ["demand_9"],
        "variants": [
            "投标截止时间为2026年9月20日9时，同时规定2026年9月18日17时后提交的文件一律拒收。",
            "开标时间注明为2026年10月12日10时，另一处又规定同日上午9时完成开标。",
            "质疑截止日为2026年8月30日，但条款同时要求所有质疑须在2026年8月28日前送达。",
        ],
    },
    {
        "code": "M05_UNCLEAR_PENALTY",
        "name": "违约责任口径不清",
        "severity": "medium",
        "critical": False,
        "aliases": ["违约责任不清", "罚款任意", "违约金无上限", "责任失衡", "处罚模糊"],
        "reason": "违约金计算基数、上限或触发条件不清，采购人可任意决定。",
        "suggestion": "明确违约事件、计算基数、比例、累计上限、通知和异议处理机制。",
        "legal": ["demand_9"],
        "variants": [
            "发生任何轻微偏差，采购人可自行决定每日处以合同总价1%至10%的违约金，累计不设上限。",
            "供应商如有采购人认为不妥的行为，应支付数额由采购人单方确定的违约金。",
            "所有违约均按合同总额的20%处罚，并可重复累计，未规定触发条件和最高限额。",
        ],
    },
]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_rows(path: Path, rows: list[dict]) -> None:
    if not rows:
        return
    with path.open("w", encoding="utf-8-sig", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)


def write_jsonl(path: Path, rows: list[dict]) -> None:
    with path.open("w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")


def download(url: str, destination: Path) -> None:
    request = urllib.request.Request(
        url,
        headers={
            "User-Agent": "Mozilla/5.0 (compatible; TenderBenchmark/1.0)",
            "Accept": "application/pdf,*/*",
        },
    )
    last_error = None
    for attempt in range(4):
        try:
            with urllib.request.urlopen(request, timeout=90) as response:
                body = response.read()
            if not body.startswith(b"%PDF"):
                raise ValueError(f"响应不是PDF，前20字节={body[:20]!r}")
            destination.write_bytes(body)
            return
        except Exception as exc:
            last_error = exc
            time.sleep(2 ** attempt)
    raise RuntimeError(str(last_error))


def wrap_text(text: str, width: int = 38) -> list[str]:
    chunks = []
    while text:
        chunks.append(text[:width])
        text = text[width:]
    return chunks or [""]


def make_injection_page(document_id: str, clauses: list[dict]) -> bytes:
    buffer = io.BytesIO()
    c = canvas.Canvas(buffer, pagesize=A4)
    pdfmetrics.registerFont(TTFont("SimHei", str(FONT_PATH)))
    c.setFont("SimHei", 15)
    c.drawString(56, 800, "附加测试条款")
    c.setFont("SimHei", 9)
    c.drawString(56, 780, f"文档编号：{document_id}　用途：标书审核基准测试（非真实采购条款）")
    y = 745
    for index, clause in enumerate(clauses, 1):
        c.setFont("SimHei", 11)
        c.drawString(56, y, f"{index}. {clause['name']}")
        y -= 22
        c.setFont("SimHei", 10)
        for line in wrap_text(clause["source_quote"]):
            c.drawString(72, y, line)
            y -= 17
        y -= 18
    c.setFont("SimHei", 8)
    c.drawString(56, 36, "该页由基准构建程序追加，仅用于检测系统识别能力，不代表原发布文件存在上述问题。")
    c.save()
    return buffer.getvalue()


def split_for(index: int) -> str:
    if index <= 30:
        return "train"
    if index <= 40:
        return "dev"
    return "test"


def main() -> int:
    for directory in (SOURCE_DIR, MUTATED_DIR, DATA_DIR):
        directory.mkdir(parents=True, exist_ok=True)
    if not FONT_PATH.exists():
        raise FileNotFoundError(f"缺少中文字体：{FONT_PATH}")

    taxonomy_rows = []
    for item in TAXONOMY:
        taxonomy_rows.append({
            "category_code": item["code"],
            "category_name": item["name"],
            "default_severity": item["severity"],
            "is_critical": item["critical"],
            "match_aliases": "|".join(item["aliases"]),
            "reason": item["reason"],
            "suggestion": item["suggestion"],
            "legal_basis": "|".join(LEGAL[key]["title"] for key in item["legal"]),
            "legal_urls": "|".join(LEGAL[key]["url"] for key in item["legal"]),
        })
    write_rows(DATA_DIR / "taxonomy.csv", taxonomy_rows)

    source_rows = []
    annotations = []
    failures = []
    critical = TAXONOMY[:5]
    high = TAXONOMY[5:10]
    medium = TAXONOMY[10:15]

    for index, (doc_id, title, purchaser, year, domain, url) in enumerate(SOURCES, 1):
        source_path = SOURCE_DIR / f"{doc_id}.pdf"
        mutated_path = MUTATED_DIR / f"{doc_id}_mutated.pdf"
        status = "ok"
        error = ""
        pages = 0
        try:
            if not source_path.exists():
                print(f"[{index:02d}/50] 下载 {doc_id} {urlparse(url).netloc}", flush=True)
                download(url, source_path)
            reader = PdfReader(str(source_path), strict=False)
            pages = len(reader.pages)
            selected = [
                critical[(index - 1) % len(critical)],
                high[(index * 2 - 1) % len(high)],
                medium[(index * 3 - 1) % len(medium)],
            ]
            clauses = []
            for ordinal, item in enumerate(selected, 1):
                variant = item["variants"][(index + ordinal) % len(item["variants"])]
                clauses.append({**item, "source_quote": variant})

            if not mutated_path.exists():
                page_reader = PdfReader(io.BytesIO(make_injection_page(doc_id, clauses)))
                writer = PdfWriter()
                for page in reader.pages:
                    writer.add_page(page)
                writer.add_page(page_reader.pages[0])
                with mutated_path.open("wb") as f:
                    writer.write(f)

            for ordinal, clause in enumerate(clauses, 1):
                legal_items = [LEGAL[key] for key in clause["legal"]]
                finding_id = f"{doc_id}-F{ordinal:02d}"
                annotations.append({
                    "benchmark_version": "silver-v1.0",
                    "split": split_for(index),
                    "document_id": doc_id,
                    "finding_id": finding_id,
                    "annotation_origin": "synthetic_injection",
                    "category_code": clause["code"],
                    "risk_type": clause["name"],
                    "severity": clause["severity"],
                    "is_critical": clause["critical"],
                    "page_number": pages + 1,
                    "section_path": ["附加测试条款"],
                    "source_quote": clause["source_quote"],
                    "reason": clause["reason"],
                    "suggestion": clause["suggestion"],
                    "legal_basis": [entry["title"] for entry in legal_items],
                    "legal_urls": [entry["url"] for entry in legal_items],
                    "match_aliases": clause["aliases"],
                    "quote_similarity_threshold": 0.45,
                    "source_file": str(source_path.relative_to(ROOT)).replace("\\", "/"),
                    "mutated_file": str(mutated_path.relative_to(ROOT)).replace("\\", "/"),
                    "review_status": "synthetic_verified",
                    "annotator": "benchmark-builder-v1",
                })
        except Exception as exc:
            status = "failed"
            error = str(exc)
            failures.append({"document_id": doc_id, "url": url, "error": error})
            print(f"[失败] {doc_id}: {error}", file=sys.stderr, flush=True)

        source_rows.append({
            "document_id": doc_id,
            "split": split_for(index),
            "title": title,
            "purchaser_or_source": purchaser,
            "publication_year": year,
            "domain": domain,
            "source_url": url,
            "source_host": urlparse(url).netloc,
            "download_status": status,
            "original_pages": pages,
            "injection_page": pages + 1 if pages else "",
            "source_sha256": sha256(source_path) if source_path.exists() else "",
            "mutated_sha256": sha256(mutated_path) if mutated_path.exists() else "",
            "source_file": str(source_path.relative_to(ROOT)).replace("\\", "/"),
            "mutated_file": str(mutated_path.relative_to(ROOT)).replace("\\", "/"),
            "redistribution_note": "公开来源，仅用于内部评测；对外发布前复核原站条款",
            "error": error,
        })

    flat_annotations = []
    for row in annotations:
        flat = dict(row)
        for key in ("section_path", "legal_basis", "legal_urls", "match_aliases"):
            flat[key] = "|".join(str(value) for value in row[key])
        flat_annotations.append(flat)

    write_rows(DATA_DIR / "source_manifest.csv", source_rows)
    write_jsonl(DATA_DIR / "source_manifest.jsonl", source_rows)
    write_rows(DATA_DIR / "annotations.csv", flat_annotations)
    write_jsonl(DATA_DIR / "annotations.jsonl", annotations)
    (DATA_DIR / "download_failures.json").write_text(
        json.dumps(failures, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    example = [{
        "document_id": "BID-001",
        "risk_id": "R_001",
        "severity": "high",
        "is_critical": True,
        "critical_reason": "地域注册限制会不合理排除外地供应商。",
        "risk_type": "地域限制",
        "source_quote": "投标人须在采购人所在地注册",
        "reason": "以注册地限制供应商",
        "suggestion": "删除地域限制",
        "page_number": next((row["page_number"] for row in annotations if row["document_id"] == "BID-001"), 1),
        "section_path": ["附加测试条款"],
        "legal_basis": ["《中华人民共和国政府采购法实施条例》第二十条"],
    }]
    write_jsonl(DATA_DIR / "predictions.example.jsonl", example)

    print(json.dumps({
        "sources": len(source_rows),
        "downloaded": sum(row["download_status"] == "ok" for row in source_rows),
        "annotations": len(annotations),
        "critical": sum(bool(row["is_critical"]) for row in annotations),
        "failures": len(failures),
    }, ensure_ascii=False, indent=2))
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
