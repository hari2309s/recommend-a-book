import * as React from "react";
import classNames from "classnames";
import { Accordion } from "radix-ui";
import { ChevronDownIcon } from "lucide-react";

interface BookDescriptionAccordionProps {
  description: string;
}

const BookDescriptionAccordion = ({ description }: BookDescriptionAccordionProps) => (
  <Accordion.Root
    className="w-[300px] rounded-md bg-green shadow-[0_2px_10px] shadow-green/5"
    type="single"
    defaultValue="item-1"
    collapsible
  >
    <AccordionItem value="item-1">
      <AccordionTrigger>{description.slice(0, 100) + '...'}</AccordionTrigger>
      <AccordionContent>
        {description}
      </AccordionContent>
    </AccordionItem>
  </Accordion.Root>
);

interface AccordionItemProps extends React.ComponentProps<typeof Accordion.Item> {
  children: React.ReactNode;
  className?: string;
  value: string;
}

const AccordionItem = React.forwardRef<HTMLDivElement, AccordionItemProps>(
  ({ children, className, ...props }, forwardedRef) => (
    <Accordion.Item
      className={classNames(
        "mt-px overflow-hidden first:mt-0 first:rounded-t last:rounded-b focus-within:relative focus-within:z-10 focus-within:shadow-[0_0_0_2px] focus-within:shadow-green",
        className,
      )}
      {...props}
      ref={forwardedRef}
    >
      {children}
    </Accordion.Item>
  ),
);

interface AccordionTriggerProps extends React.ComponentProps<typeof Accordion.Trigger> {
  children: React.ReactNode;
  className?: string;
}

const AccordionTrigger = React.forwardRef<HTMLButtonElement, AccordionTriggerProps>(
  ({ children, className, ...props }, forwardedRef) => (
    <Accordion.Header className="flex">
      <Accordion.Trigger
        className={classNames(
          "group flex h-[55px] flex-1 cursor-default items-center justify-between bg-green px-5 text-[15px] leading-none text-accent-11 shadow-[0_1px_0] shadow-green outline-none hover:bg-green",
          className,
        )}
        {...props}
        ref={forwardedRef}
      >
        {children}
        <ChevronDownIcon
          className="text-accent-10 transition-transform duration-300 ease-[cubic-bezier(0.87,_0,_0.13,_1)] group-data-[state=open]:rotate-180"
          aria-hidden
        />
      </Accordion.Trigger>
    </Accordion.Header>
  ),
);

interface AccordionContentProps {
  children: React.ReactNode;
  className?: string;
  [key: string]: any;
}

const AccordionContent = React.forwardRef<HTMLDivElement, AccordionContentProps>(
  ({ children, className, ...props }, forwardedRef) => (
    <Accordion.Content
      className={classNames(
        "overflow-hidden bg-green text-[15px] text-accent-11 data-[state=closed]:animate-slideUp data-[state=open]:animate-slideDown",
        className,
      )}
      {...props}
      ref={forwardedRef}
    >
      <div className="px-5 py-[15px]">{children}</div>
    </Accordion.Content>
  ),
);

export default BookDescriptionAccordion;
