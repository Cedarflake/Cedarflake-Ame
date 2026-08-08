import "package:flutter/material.dart";

class VerticalMaterialTimelineSlider extends StatefulWidget {
  const VerticalMaterialTimelineSlider({
    required this.value,
    required this.endpointInset,
    required this.onChanged,
    required this.onChangeEnd,
    required this.semanticLabelFor,
    super.key,
  });

  final double value;
  final double endpointInset;
  final ValueChanged<double> onChanged;
  final ValueChanged<double> onChangeEnd;
  final String Function(double value) semanticLabelFor;

  @override
  State<VerticalMaterialTimelineSlider> createState() =>
      _VerticalMaterialTimelineSliderState();
}

class _VerticalMaterialTimelineSliderState
    extends State<VerticalMaterialTimelineSlider> {
  final FocusNode _focusNode = FocusNode(
    debugLabel: "Annotated timeline Material Slider",
  );

  @override
  void dispose() {
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final sliderTheme = SliderTheme.of(context).copyWith(
      trackHeight: 1,
      activeTrackColor: Colors.transparent,
      inactiveTrackColor: Colors.transparent,
      trackShape: const RoundedRectSliderTrackShape(),
      thumbShape: SliderComponentShape.noThumb,
      overlayShape: SliderComponentShape.noOverlay,
    );
    return Stack(
      fit: StackFit.expand,
      children: [
        Center(
          child: SizedBox(
            key: const Key("timeline-track-background"),
            width: 16,
            height: double.infinity,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: colorScheme.secondaryContainer,
                borderRadius: BorderRadius.circular(8),
              ),
            ),
          ),
        ),
        SliderTheme(
          data: sliderTheme,
          child: RotatedBox(
            quarterTurns: 3,
            child: Slider(
              key: const Key("timeline-slider"),
              value: 1 - widget.value.clamp(0.0, 1.0).toDouble(),
              onChanged: (value) => widget.onChanged(1 - value),
              onChangeEnd: (value) => widget.onChangeEnd(1 - value),
              allowedInteraction: SliderInteraction.tapAndSlide,
              focusNode: _focusNode,
              padding: EdgeInsets.symmetric(horizontal: widget.endpointInset),
              semanticFormatterCallback: (value) =>
                  widget.semanticLabelFor(1 - value),
            ),
          ),
        ),
      ],
    );
  }
}
